use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::config::Config;

use super::test_server::spawn_test_server_on;
use super::{AppState, PeerClient};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const PRESET: &str = "split";

struct TwoInstanceHarness {
    peer_addr: SocketAddr,
    coord_addr: SocketAddr,
    peer_server: JoinHandle<()>,
    coord_server: JoinHandle<()>,
}

impl TwoInstanceHarness {
    fn coordinator_client(&self) -> PeerClient {
        PeerClient::new("127.0.0.1", self.coord_addr.port())
    }

    fn peer_client(&self) -> PeerClient {
        PeerClient::new("127.0.0.1", self.peer_addr.port())
    }
}

async fn spawn_harness_on(
    peer_listener: TcpListener,
    coord_listener: TcpListener,
    peer_config: Config,
    coord_config: Config,
) -> TwoInstanceHarness {
    let (peer_addr, peer_server) =
        spawn_test_server_on(peer_listener, AppState::new(Some(peer_config))).await;
    let (coord_addr, coord_server) =
        spawn_test_server_on(coord_listener, AppState::new(Some(coord_config))).await;

    TwoInstanceHarness {
        peer_addr,
        coord_addr,
        peer_server,
        coord_server,
    }
}

impl Drop for TwoInstanceHarness {
    fn drop(&mut self) {
        self.peer_server.abort();
        self.coord_server.abort();
    }
}

fn peer_config(monitor2_command: &str, coordinator_port: Option<u16>) -> Config {
    let peers = match coordinator_port {
        Some(port) => {
            format!(r#"[{{ "name": "windows-pc", "host": "127.0.0.1", "port": {port} }}]"#)
        }
        None => "[]".to_string(),
    };

    let monitor1_stub = match coordinator_port {
        Some(_) => {
            r#",
                {
                    "id": "monitor1",
                    "label": "Coordinator Monitor",
                    "order": 0,
                    "controlledBy": "windows-pc"
                }"#
        }
        None => "",
    };

    let json = format!(
        r#"{{
            "deviceName": "mac-mini",
            "peers": {peers},
            "devices": [
                {{ "id": "windows-pc", "label": "Windows PC" }},
                {{ "id": "mac-mini", "label": "Mac mini" }}
            ],
            "monitors": [
                {{
                    "id": "monitor2",
                    "label": "Remote Monitor",
                    "order": 1,
                    "inputs": {{
                        "mac-mini": {{ "type": "hdmi", "command": "{monitor2_command}" }}
                    }}
                }}{monitor1_stub}
            ],
            "presets": {{
                "split": {{
                    "label": "Split",
                    "layout": {{ "monitor1": "windows-pc", "monitor2": "mac-mini" }}
                }}
            }}
        }}"#
    );
    serde_json::from_str(&json).expect("peer fixture config should parse")
}

fn coordinator_config(peer_port: u16) -> Config {
    let json = format!(
        r#"{{
            "deviceName": "windows-pc",
            "peers": [
                {{ "name": "mac-mini", "host": "127.0.0.1", "port": {peer_port} }}
            ],
            "devices": [
                {{ "id": "windows-pc", "label": "Windows PC" }},
                {{ "id": "mac-mini", "label": "Mac mini" }}
            ],
            "monitors": [
                {{
                    "id": "monitor1",
                    "label": "Local Monitor",
                    "order": 0,
                    "inputs": {{
                        "windows-pc": {{ "type": "hdmi", "command": "exit 0" }}
                    }}
                }},
                {{
                    "id": "monitor2",
                    "label": "Remote Monitor",
                    "order": 1,
                    "controlledBy": "mac-mini"
                }}
            ],
            "presets": {{
                "split": {{
                    "label": "Split",
                    "layout": {{ "monitor1": "windows-pc", "monitor2": "mac-mini" }}
                }}
            }}
        }}"#
    );
    serde_json::from_str(&json).expect("coordinator fixture config should parse")
}

async fn spawn_standard_harness(monitor2_command: &str) -> TwoInstanceHarness {
    let peer_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind peer port");
    let peer_port = peer_listener.local_addr().expect("peer local addr").port();
    let coord_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind coordinator port");

    spawn_harness_on(
        peer_listener,
        coord_listener,
        peer_config(monitor2_command, None),
        coordinator_config(peer_port),
    )
    .await
}

async fn fetch_last_applied_preset(client: &PeerClient) -> Option<String> {
    let status = client.status().await.expect("status should succeed");
    status.last_applied_preset
}

#[tokio::test]
async fn coordinated_dry_run_end_to_end() {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let harness = spawn_standard_harness("exit 0").await;
        let client = harness.coordinator_client();

        let response = client
            .apply_preset(PRESET, true, false)
            .await
            .expect("coordinated dry-run should succeed");

        assert!(response.dry_run);
        assert!(!response.local_only);
        assert!(response.planning_errors.is_empty());
        assert_eq!(response.local_results.len(), 1);
        assert_eq!(response.local_results[0].monitor_id, "monitor1");
        assert_eq!(
            response.local_results[0].outcome,
            crate::executor::MonitorOutcome::DryRun
        );

        assert_eq!(response.peer_results.len(), 1);
        assert_eq!(response.peer_results[0].device_id, "mac-mini");
        let crate::orchestrator::PeerOutcome::Success {
            local_only,
            results,
            peer_results,
        } = &response.peer_results[0].outcome
        else {
            panic!("expected peer success outcome");
        };
        assert!(local_only);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].monitor_id, "monitor2");
        assert_eq!(results[0].outcome, crate::executor::MonitorOutcome::DryRun);
        assert!(peer_results.is_empty());

        assert!(fetch_last_applied_preset(&client).await.is_none());
        assert!(fetch_last_applied_preset(&harness.peer_client())
            .await
            .is_none());
    })
    .await
    .expect("coordinated dry-run test timed out");
}

#[tokio::test]
async fn coordinated_apply_full_success_updates_last_applied_preset() {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let harness = spawn_standard_harness("exit 0").await;
        let client = harness.coordinator_client();

        let response = client
            .apply_preset(PRESET, false, false)
            .await
            .expect("coordinated apply should succeed");

        assert!(!response.dry_run);
        assert_eq!(response.local_results.len(), 1);
        assert!(matches!(
            response.local_results[0].outcome,
            crate::executor::MonitorOutcome::Success { .. }
        ));

        let crate::orchestrator::PeerOutcome::Success { results, .. } =
            &response.peer_results[0].outcome
        else {
            panic!("expected peer success outcome");
        };
        assert!(matches!(
            results[0].outcome,
            crate::executor::MonitorOutcome::Success { .. }
        ));

        assert_eq!(
            fetch_last_applied_preset(&client).await.as_deref(),
            Some(PRESET)
        );
        assert_eq!(
            fetch_last_applied_preset(&harness.peer_client())
                .await
                .as_deref(),
            Some(PRESET)
        );
    })
    .await
    .expect("coordinated full-success test timed out");
}

#[tokio::test]
async fn coordinated_apply_peer_failure_blocks_last_applied_preset() {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let harness = spawn_standard_harness("exit 1").await;
        let client = harness.coordinator_client();

        let response = client
            .apply_preset(PRESET, false, false)
            .await
            .expect("coordinated apply should return a response");

        assert!(matches!(
            response.local_results[0].outcome,
            crate::executor::MonitorOutcome::Success { .. }
        ));

        let crate::orchestrator::PeerOutcome::Success { results, .. } =
            &response.peer_results[0].outcome
        else {
            panic!("expected structured peer success wrapper with failed nested result");
        };
        assert!(matches!(
            results[0].outcome,
            crate::executor::MonitorOutcome::Failed { .. }
        ));

        assert!(fetch_last_applied_preset(&client).await.is_none());
        assert!(fetch_last_applied_preset(&harness.peer_client())
            .await
            .is_none());
    })
    .await
    .expect("peer failure test timed out");
}

#[tokio::test]
async fn peer_does_not_recursively_fan_out_from_coordinator() {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let peer_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind peer port");
        let coord_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind coordinator port");
        let peer_port = peer_listener.local_addr().expect("peer addr").port();
        let coord_port = coord_listener.local_addr().expect("coord addr").port();

        let peer_config = peer_config("exit 0", Some(coord_port));
        let coord_config = coordinator_config(peer_port);

        let harness =
            spawn_harness_on(peer_listener, coord_listener, peer_config, coord_config).await;

        let peer_direct = harness.peer_client();
        let fanout_response = peer_direct
            .apply_preset(PRESET, true, false)
            .await
            .expect("peer coordinated dry-run should succeed");
        assert!(
            !fanout_response.peer_results.is_empty(),
            "negative control: peer with localOnly=false should fan out to coordinator"
        );

        let coordinator = harness.coordinator_client();
        let response = coordinator
            .apply_preset(PRESET, true, false)
            .await
            .expect("coordinator dry-run should succeed");

        let crate::orchestrator::PeerOutcome::Success {
            local_only,
            peer_results,
            ..
        } = &response.peer_results[0].outcome
        else {
            panic!("expected peer success outcome");
        };
        assert!(local_only);
        assert!(
            peer_results.is_empty(),
            "peer should not fan out when coordinator sends localOnly=true"
        );

        assert!(fetch_last_applied_preset(&coordinator).await.is_none());
        assert!(fetch_last_applied_preset(&peer_direct).await.is_none());
    })
    .await
    .expect("recursion guard test timed out");
}
