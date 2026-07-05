use std::sync::Arc;

use crate::executor::ExecutorError;
use crate::orchestrator::{apply_preset, CoordinatedApplyResult, PeerClientAdapter};

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
) -> Result<CoordinatedApplyResult, ApplyPresetStateError> {
    let config = state
        .config
        .as_ref()
        .ok_or(ApplyPresetStateError::ConfigNotLoaded)?;

    let peer_client = PeerClientAdapter;
    match apply_preset(config, preset, dry_run, local_only, &peer_client).await {
        Ok(result) => {
            if !dry_run && result.is_full_success() {
                *state
                    .last_applied_preset
                    .lock()
                    .expect("last_applied_preset lock poisoned") = Some(preset.to_string());
            }
            Ok(result)
        }
        Err(ExecutorError::PresetNotFound { preset_name }) => {
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
) -> Result<CoordinatedApplyResult, ApplyPresetStateError> {
    apply_preset_to_state(&state, preset, dry_run, local_only).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::config::Config;

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
        apply_preset_to_state(&state, "all_a", true, true)
            .await
            .expect("dry run should succeed");
        assert!(state.last_applied_preset.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn successful_apply_updates_last_applied_preset() {
        let state = Arc::new(AppState::from_load_result(Ok(test_config())));
        apply_preset_to_state(&state, "all_a", false, true)
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
        let err = apply_preset_to_state(&state, "all_a", false, false)
            .await
            .expect_err("should fail without config");
        assert_eq!(err, ApplyPresetStateError::ConfigNotLoaded);
    }
}
