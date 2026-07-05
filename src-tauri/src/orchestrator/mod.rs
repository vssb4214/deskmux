mod model;
mod peer;
mod plan;

pub use model::{
    CoordinatedApplyResult, PeerApplyOutcome, PeerOutcome, PeerRef, PlanningError,
    PlanningErrorKind,
};
pub use peer::{PeerApplyClient, PeerApplyResponse, PeerClientAdapter};
pub use plan::{plan_apply, ApplyPlan, ApplyScope};

use crate::api::PeerClientError;
use crate::config::{Config, Peer};
use crate::executor::{apply_layout_entries, ExecutorError};

pub async fn apply_preset(
    config: &Config,
    preset_name: &str,
    dry_run: bool,
    local_only: bool,
    peer_client: &dyn PeerApplyClient,
) -> Result<CoordinatedApplyResult, ExecutorError> {
    let scope = if local_only {
        ApplyScope::LocalOnly
    } else {
        ApplyScope::Coordinated
    };

    let plan = plan_apply(config, preset_name, scope)?;
    let local_results = apply_layout_entries(config, &plan.local_entries, dry_run);

    let mut peer_results = Vec::new();
    if !local_only {
        for device_id in &plan.remote_peers {
            peer_results
                .push(apply_on_peer(config, peer_client, device_id, preset_name, dry_run).await);
        }
    }

    Ok(CoordinatedApplyResult {
        preset: preset_name.to_string(),
        dry_run,
        local_only,
        planning_errors: plan.planning_errors,
        local_results,
        peer_results,
    })
}

async fn apply_on_peer(
    config: &Config,
    peer_client: &dyn PeerApplyClient,
    device_id: &str,
    preset_name: &str,
    dry_run: bool,
) -> PeerApplyOutcome {
    let Some(peer) = find_peer(config, device_id) else {
        return PeerApplyOutcome {
            device_id: device_id.to_string(),
            peer: None,
            outcome: PeerOutcome::Failed {
                error: format!("no peer configured for device '{device_id}'"),
                http_status: None,
            },
        };
    };

    let peer_ref = PeerRef {
        host: peer.host.clone(),
        port: peer.port,
    };

    match peer_client
        .apply_preset_local(&peer.host, peer.port, preset_name, dry_run)
        .await
    {
        Ok(response) => PeerApplyOutcome {
            device_id: device_id.to_string(),
            peer: Some(peer_ref),
            outcome: PeerOutcome::Success {
                local_only: response.local_only,
                results: response.local_results,
                peer_results: response.peer_results,
            },
        },
        Err(PeerClientError::Http { status, error }) => PeerApplyOutcome {
            device_id: device_id.to_string(),
            peer: Some(peer_ref),
            outcome: PeerOutcome::Failed {
                error,
                http_status: Some(status.as_u16()),
            },
        },
        Err(PeerClientError::Request(err)) => PeerApplyOutcome {
            device_id: device_id.to_string(),
            peer: Some(peer_ref),
            outcome: PeerOutcome::Failed {
                error: err.to_string(),
                http_status: None,
            },
        },
    }
}

fn find_peer<'a>(config: &'a Config, device_id: &str) -> Option<&'a Peer> {
    config.peers.iter().find(|peer| peer.name == device_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{MonitorOutcome, MonitorResult};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    type PeerCallLog = (String, u16, String, bool);

    struct MockPeerApplyClient {
        calls: Arc<Mutex<Vec<PeerCallLog>>>,
        responses: Mutex<HashMap<String, Result<PeerApplyResponse, PeerClientError>>>,
    }

    impl MockPeerApplyClient {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                responses: Mutex::new(HashMap::new()),
            }
        }

        fn queue(&self, device_id: &str, response: Result<PeerApplyResponse, PeerClientError>) {
            self.responses
                .lock()
                .expect("lock")
                .insert(device_id.to_string(), response);
        }

        fn call_count(&self) -> usize {
            self.calls.lock().expect("lock").len()
        }

        fn last_call_dry_run(&self) -> bool {
            self.calls.lock().expect("lock").last().unwrap().3
        }
    }

    #[async_trait]
    impl PeerApplyClient for MockPeerApplyClient {
        async fn apply_preset_local(
            &self,
            host: &str,
            port: u16,
            preset: &str,
            dry_run: bool,
        ) -> Result<PeerApplyResponse, PeerClientError> {
            self.calls.lock().expect("lock").push((
                host.to_string(),
                port,
                preset.to_string(),
                dry_run,
            ));
            let device_id = host; // tests key by host for simplicity
            self.responses
                .lock()
                .expect("lock")
                .remove(device_id)
                .unwrap_or_else(|| {
                    Err(PeerClientError::Http {
                        status: reqwest::StatusCode::NOT_FOUND,
                        error: "missing mock".to_string(),
                    })
                })
        }
    }

    fn coordinator_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [{ "name": "device-b", "host": "192.168.1.2", "port": 3737 }],
            "devices": [
                { "id": "device-a", "label": "A" },
                { "id": "device-b", "label": "B" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "M1",
                    "order": 0,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-m1-a" }
                    }
                },
                {
                    "id": "monitor2",
                    "label": "M2",
                    "order": 1,
                    "controlledBy": "device-b"
                }
            ],
            "presets": {
                "split": {
                    "label": "Split",
                    "layout": { "monitor1": "device-a", "monitor2": "device-b" }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    fn peer_success_result() -> PeerApplyResponse {
        PeerApplyResponse {
            local_only: true,
            local_results: vec![MonitorResult {
                monitor_id: "monitor2".to_string(),
                device_id: "device-b".to_string(),
                command: Some("cmd".to_string()),
                executed: false,
                is_native_ddc: false,
                outcome: MonitorOutcome::DryRun,
            }],
            peer_results: vec![],
        }
    }

    #[tokio::test]
    async fn local_only_preset_does_not_call_peer() {
        let config = coordinator_config();
        let mock = MockPeerApplyClient::new();

        let result = apply_preset(&config, "split", false, true, &mock)
            .await
            .expect("apply should succeed");

        assert_eq!(mock.call_count(), 0);
        assert_eq!(result.local_results.len(), 1);
        assert!(result.peer_results.is_empty());
    }

    #[tokio::test]
    async fn coordinated_apply_calls_peer_with_local_only_and_dry_run() {
        let config = coordinator_config();
        let mock = MockPeerApplyClient::new();
        mock.queue("192.168.1.2", Ok(peer_success_result()));

        let result = apply_preset(&config, "split", true, false, &mock)
            .await
            .expect("apply should succeed");

        assert_eq!(mock.call_count(), 1);
        assert!(mock.last_call_dry_run());
        assert_eq!(result.local_results.len(), 1);
        assert!(result.local_results[0].outcome == MonitorOutcome::DryRun);
        assert_eq!(result.peer_results.len(), 1);
        assert!(result.is_full_success());
    }

    #[tokio::test]
    async fn peer_http_error_is_structured_without_panic() {
        let config = coordinator_config();
        let mock = MockPeerApplyClient::new();
        mock.queue(
            "192.168.1.2",
            Err(PeerClientError::Http {
                status: reqwest::StatusCode::NOT_FOUND,
                error: "preset missing".to_string(),
            }),
        );

        let result = apply_preset(&config, "split", false, false, &mock)
            .await
            .expect("apply should succeed");

        assert!(!result.is_full_success());
        assert!(matches!(
            result.peer_results[0].outcome,
            PeerOutcome::Failed { .. }
        ));
    }

    #[tokio::test]
    async fn missing_peer_config_is_structured_error() {
        let mut config = coordinator_config();
        config.peers.clear();

        let mock = MockPeerApplyClient::new();
        let result = apply_preset(&config, "split", false, false, &mock)
            .await
            .expect("apply should succeed");

        assert_eq!(mock.call_count(), 0);
        assert!(matches!(
            result.peer_results[0].outcome,
            PeerOutcome::Failed { .. }
        ));
        assert!(!result.is_full_success());
    }

    #[tokio::test]
    async fn planning_error_prevents_full_success() {
        let mut config = coordinator_config();
        config
            .presets
            .get_mut("split")
            .unwrap()
            .layout
            .insert("ghost".to_string(), "device-a".to_string());

        let mock = MockPeerApplyClient::new();
        mock.queue("192.168.1.2", Ok(peer_success_result()));

        let result = apply_preset(&config, "split", false, false, &mock)
            .await
            .expect("apply should succeed");

        assert!(!result.planning_errors.is_empty());
        assert!(!result.is_full_success());
    }
}
