use std::sync::Arc;

use crate::executor::MonitorOutcome;
use crate::orchestrator::{apply_preset, CoordinatedApplyResult, PeerClientAdapter};

use super::events::{
    record_apply_finished, record_apply_started, record_native_ddc_result, ApplySource,
};
use super::handlers::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyPresetStateError {
    ConfigNotLoaded,
    PresetNotFound { preset_name: String },
}

/// Applies a preset through the shared app state (API, tray, and hotkeys).
///
/// Updates `last_applied_preset` only on non-dry-run full success.
pub async fn apply_preset_to_state(
    state: &AppState,
    preset: &str,
    dry_run: bool,
    local_only: bool,
    source: ApplySource,
) -> Result<CoordinatedApplyResult, ApplyPresetStateError> {
    let config = state
        .config
        .as_ref()
        .ok_or(ApplyPresetStateError::ConfigNotLoaded)?;

    record_apply_started(&state.events, preset, dry_run, source);

    let peer_client = PeerClientAdapter;
    match apply_preset(config, preset, dry_run, local_only, &peer_client).await {
        Ok(result) => {
            record_apply_outcome(state, preset, dry_run, source, &result);
            if !dry_run && result.is_full_success() {
                *state
                    .last_applied_preset
                    .lock()
                    .expect("last_applied_preset lock poisoned") = Some(preset.to_string());
            }
            Ok(result)
        }
        Err(crate::executor::ExecutorError::PresetNotFound { preset_name }) => {
            record_apply_finished(&state.events, preset, dry_run, source, false, &[]);
            Err(ApplyPresetStateError::PresetNotFound { preset_name })
        }
    }
}

/// Convenience wrapper for callers that hold `Arc<AppState>`.
pub async fn apply_preset_to_arc(
    state: Arc<AppState>,
    preset: &str,
    dry_run: bool,
    local_only: bool,
    source: ApplySource,
) -> Result<CoordinatedApplyResult, ApplyPresetStateError> {
    apply_preset_to_state(&state, preset, dry_run, local_only, source).await
}

fn record_apply_outcome(
    state: &AppState,
    preset: &str,
    dry_run: bool,
    source: ApplySource,
    result: &CoordinatedApplyResult,
) {
    let failed_monitors = failed_monitor_ids(result);
    record_apply_finished(
        &state.events,
        preset,
        dry_run,
        source,
        result.is_full_success(),
        &failed_monitors,
    );
    for monitor_result in &result.local_results {
        if monitor_result.is_native_ddc {
            let success = monitor_outcome_ok(&monitor_result.outcome);
            record_native_ddc_result(
                &state.events,
                &monitor_result.monitor_id,
                success,
                Some(preset),
                Some(source),
            );
        }
    }
}

fn monitor_outcome_ok(outcome: &MonitorOutcome) -> bool {
    matches!(
        outcome,
        MonitorOutcome::Success { .. } | MonitorOutcome::DryRun
    )
}

fn failed_monitor_ids(result: &CoordinatedApplyResult) -> Vec<String> {
    let mut failed = Vec::new();
    for r in &result.local_results {
        if !monitor_outcome_ok(&r.outcome) {
            failed.push(r.monitor_id.clone());
        }
    }
    for peer in &result.peer_results {
        if let crate::orchestrator::PeerOutcome::Failed { .. } = peer.outcome {
            failed.push(format!("peer:{}", peer.device_id));
        } else if let crate::orchestrator::PeerOutcome::Success { results, .. } = &peer.outcome {
            for r in results {
                if !monitor_outcome_ok(&r.outcome) {
                    failed.push(format!("peer:{}:{}", peer.device_id, r.monitor_id));
                }
            }
        }
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::api::events::{EventKind, MAX_EVENTS};
    use crate::config::Config;
    use crate::executor::MonitorResult;

    fn test_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [
                { "id": "device-a", "label": "Device A" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "Monitor 1",
                    "order": 0,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "exit 0" }
                    }
                }
            ],
            "presets": {
                "all_a": { "label": "All A", "layout": { "monitor1": "device-a" } }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    #[tokio::test]
    async fn dry_run_does_not_update_last_applied_preset() {
        let state = Arc::new(AppState::from_load_result(Ok(test_config())));
        apply_preset_to_state(&state, "all_a", true, true, ApplySource::Api)
            .await
            .expect("dry run should succeed");
        assert!(state.last_applied_preset.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn successful_apply_updates_last_applied_preset() {
        let state = Arc::new(AppState::from_load_result(Ok(test_config())));
        apply_preset_to_state(&state, "all_a", false, true, ApplySource::Api)
            .await
            .expect("apply should succeed");
        assert_eq!(
            state.last_applied_preset.lock().unwrap().as_deref(),
            Some("all_a")
        );
    }

    #[tokio::test]
    async fn missing_config_returns_error() {
        let state = Arc::new(AppState::from_load_result(Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing",
        )
        .into())));
        let err = apply_preset_to_state(&state, "all_a", false, false, ApplySource::Api)
            .await
            .expect_err("should fail without config");
        assert_eq!(err, ApplyPresetStateError::ConfigNotLoaded);
    }

    #[tokio::test]
    async fn apply_records_start_and_finish_events() {
        let state = Arc::new(AppState::from_load_result(Ok(test_config())));
        apply_preset_to_state(&state, "all_a", true, true, ApplySource::Api)
            .await
            .expect("dry run should succeed");

        let events = state.events.lock().unwrap().recent(MAX_EVENTS);
        assert!(events.iter().any(|e| e.message.contains("started")));
        assert!(events.iter().any(|e| e.kind == EventKind::Success));
    }

    /// Regression guard: native-DDC event recording must key off `MonitorResult.is_native_ddc`,
    /// not sniff the `command` display string for a `"native DDC:"` prefix. This result's
    /// `command` is deliberately reworded — if detection ever regresses to string-sniffing
    /// (`backend.rs`'s `display_command` format changes, or someone reintroduces
    /// `starts_with`), this test fails while the field-based check keeps working.
    #[tokio::test]
    async fn native_ddc_event_fires_even_if_display_string_is_reworded() {
        let state = Arc::new(AppState::from_load_result(Ok(test_config())));
        let result = CoordinatedApplyResult {
            preset: "all_a".to_string(),
            dry_run: false,
            local_only: true,
            planning_errors: vec![],
            local_results: vec![MonitorResult {
                monitor_id: "monitor1".to_string(),
                device_id: "device-a".to_string(),
                command: Some("totally reworded display text, no magic prefix".to_string()),
                executed: true,
                is_native_ddc: true,
                outcome: MonitorOutcome::Success {
                    stdout: String::new(),
                    stderr: String::new(),
                },
            }],
            peer_results: vec![],
        };

        record_apply_outcome(&state, "all_a", false, ApplySource::Api, &result);

        let events = state.events.lock().unwrap().recent(MAX_EVENTS);
        assert!(events
            .iter()
            .any(|e| e.message.contains("Native DDC input switch succeeded")));
    }
}
