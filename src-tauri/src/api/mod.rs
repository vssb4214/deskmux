pub mod apply;
pub mod bind;
pub mod client;
mod cors;
pub mod discovery;
pub mod events;
mod handlers;
#[cfg(test)]
mod peer_orchestration;
mod server;
#[cfg(test)]
mod test_server;
pub mod types;

pub use apply::{apply_preset_to_arc, apply_preset_to_state, ApplyPresetStateError};
pub use bind::{dashboard_api_base_url, resolve_bind_addr, DEFAULT_BIND_HOST, DEFAULT_PORT};
pub use client::{PeerClient, PeerClientError};
pub use events::{ApplySource, DeskMuxEvent, EventLog};
pub use handlers::AppState;
pub use server::spawn_server;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/events", get(handlers::events))
        .route("/apply-preset", post(handlers::apply_preset_handler))
        .route(
            "/native-ddc/displays",
            get(discovery::list_displays_handler),
        )
        .route(
            "/native-ddc/displays/{display_id}/input-source",
            get(discovery::read_input_source_handler),
        )
        .route(
            "/native-ddc/displays/{display_id}/probe-input",
            post(discovery::probe_input_handler),
        )
        .layer(cors::dashboard_cors_layer())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, HeaderValue, Request, StatusCode};
    use axum::Router;
    use tower::ServiceExt;

    use crate::config::Config;
    use crate::executor::discovery::{DiscoveredDisplay, DiscoveryError, InputSourceReading};
    use discovery::DiscoverySource;

    fn app_with_config() -> Router {
        router(Arc::new(AppState::from_load_result(Ok(test_config()))))
    }

    fn app_without_config() -> Router {
        router(Arc::new(AppState::from_load_result(Err(
            std::io::Error::new(std::io::ErrorKind::NotFound, "deskmux.config.json").into(),
        ))))
    }

    /// Scripted discovery source so these tests are deterministic on every CI platform — the
    /// real source would need actual display hardware on Windows and is a stub elsewhere.
    struct MockDiscovery {
        native_available: bool,
        displays: Vec<String>,
        read_behavior: ReadBehavior,
        probe_behavior: ProbeBehavior,
    }

    enum ReadBehavior {
        /// Ok with this reading when the display is in `displays`; DisplayNotFound otherwise.
        Reading(InputSourceReading),
        /// Always this error, regardless of display.
        Fail(DiscoveryError),
    }

    enum ProbeBehavior {
        /// Ok for displays present in `displays`; DisplayNotFound otherwise.
        Accept { current: Option<u32> },
        /// Always this error, regardless of display.
        Fail(DiscoveryError),
    }

    impl MockDiscovery {
        fn available(displays: Vec<&str>, reading: InputSourceReading) -> Self {
            Self {
                native_available: true,
                displays: displays.into_iter().map(str::to_string).collect(),
                read_behavior: ReadBehavior::Reading(reading),
                probe_behavior: ProbeBehavior::Accept { current: None },
            }
        }

        fn unavailable() -> Self {
            Self {
                native_available: false,
                displays: Vec::new(),
                read_behavior: ReadBehavior::Fail(DiscoveryError::NativeUnavailable),
                probe_behavior: ProbeBehavior::Fail(DiscoveryError::NativeUnavailable),
            }
        }

        fn failing_reads(displays: Vec<&str>, error: DiscoveryError) -> Self {
            Self {
                native_available: true,
                displays: displays.into_iter().map(str::to_string).collect(),
                read_behavior: ReadBehavior::Fail(error),
                probe_behavior: ProbeBehavior::Accept { current: None },
            }
        }

        fn failing_probe(displays: Vec<&str>, error: DiscoveryError) -> Self {
            Self {
                native_available: true,
                displays: displays.into_iter().map(str::to_string).collect(),
                read_behavior: ReadBehavior::Reading(READING_4626),
                probe_behavior: ProbeBehavior::Fail(error),
            }
        }

        fn available_with_probe_current(displays: Vec<&str>, current: Option<u32>) -> Self {
            Self {
                native_available: true,
                displays: displays.into_iter().map(str::to_string).collect(),
                read_behavior: ReadBehavior::Reading(READING_4626),
                probe_behavior: ProbeBehavior::Accept { current },
            }
        }
    }

    impl DiscoverySource for MockDiscovery {
        fn native_available(&self) -> bool {
            self.native_available
        }

        fn list_displays(&self) -> Result<Vec<DiscoveredDisplay>, DiscoveryError> {
            Ok(self
                .displays
                .iter()
                .map(|id| DiscoveredDisplay {
                    display_id: id.clone(),
                })
                .collect())
        }

        fn read_input_source(
            &self,
            display_id: &str,
        ) -> Result<InputSourceReading, DiscoveryError> {
            match &self.read_behavior {
                ReadBehavior::Fail(error) => Err(error.clone()),
                ReadBehavior::Reading(reading) => {
                    if self.displays.iter().any(|d| d == display_id) {
                        Ok(*reading)
                    } else {
                        Err(DiscoveryError::DisplayNotFound {
                            display_id: display_id.to_string(),
                        })
                    }
                }
            }
        }

        fn probe_input(
            &self,
            display_id: &str,
            _value: u16,
        ) -> Result<crate::executor::discovery::ProbeInputResult, DiscoveryError> {
            match &self.probe_behavior {
                ProbeBehavior::Fail(error) => Err(error.clone()),
                ProbeBehavior::Accept { current } => {
                    if self.displays.iter().any(|d| d == display_id) {
                        Ok(crate::executor::discovery::ProbeInputResult {
                            accepted: true,
                            current: *current,
                        })
                    } else {
                        Err(DiscoveryError::DisplayNotFound {
                            display_id: display_id.to_string(),
                        })
                    }
                }
            }
        }
    }

    fn app_with_discovery(mock: MockDiscovery) -> Router {
        router(Arc::new(AppState::with_discovery(
            Ok(test_config()),
            Box::new(mock),
        )))
    }

    const READING_4626: InputSourceReading = InputSourceReading {
        current: 4626,
        maximum: 4626,
    };

    #[tokio::test]
    async fn discovery_displays_lists_native_displays() {
        let app = app_with_discovery(MockDiscovery::available(
            vec!["K@P:d0e5:0", "KJL:0e25:2"],
            READING_4626,
        ));

        let (status, json) = get(&app, "/native-ddc/displays").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["nativeAvailable"], true);
        assert_eq!(json["displays"][0]["displayId"], "K@P:d0e5:0");
        assert_eq!(json["displays"][1]["displayId"], "KJL:0e25:2");
    }

    #[tokio::test]
    async fn discovery_displays_honest_when_native_unavailable() {
        let app = app_with_discovery(MockDiscovery::unavailable());

        let (status, json) = get(&app, "/native-ddc/displays").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["nativeAvailable"], false);
        assert!(json["displays"].as_array().unwrap().is_empty());
    }

    /// First-run means no config exists — discovery must work anyway (like /events).
    #[tokio::test]
    async fn discovery_displays_works_without_config() {
        let app = router(Arc::new(AppState::with_discovery(
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "deskmux.config.json").into()),
            Box::new(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626)),
        )));

        let (status, json) = get(&app, "/native-ddc/displays").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["displays"][0]["displayId"], "K@P:d0e5:0");
    }

    /// displayIds contain `@` and `:`; the dashboard sends them percent-encoded and axum must
    /// decode back to the exact id the executor knows.
    #[tokio::test]
    async fn discovery_input_source_reads_percent_encoded_display_id() {
        let app = app_with_discovery(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626));

        let (status, json) = get(&app, "/native-ddc/displays/K%40P%3Ad0e5%3A0/input-source").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["current"], 4626);
        assert_eq!(json["maximum"], 4626);
    }

    #[tokio::test]
    async fn discovery_input_source_unknown_display_is_404_with_code() {
        let app = app_with_discovery(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626));

        let (status, json) = get(&app, "/native-ddc/displays/GHOST%3A0000%3A0/input-source").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["code"], "displayNotFound");
        assert!(json["error"].as_str().unwrap().contains("GHOST:0000:0"));
    }

    #[tokio::test]
    async fn discovery_input_source_read_failure_is_500_with_code() {
        let app = app_with_discovery(MockDiscovery::failing_reads(
            vec!["KJL:0e25:2"],
            DiscoveryError::VcpReadFailed {
                detail: "no physical monitor responded; after refresh: still nothing".to_string(),
            },
        ));

        let (status, json) = get(&app, "/native-ddc/displays/KJL%3A0e25%3A2/input-source").await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["code"], "vcpReadFailed");
    }

    #[tokio::test]
    async fn discovery_input_source_unavailable_platform_is_501() {
        let app = app_with_discovery(MockDiscovery::unavailable());

        let (status, json) = get(&app, "/native-ddc/displays/K%40P%3Ad0e5%3A0/input-source").await;

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(json["code"], "nativeUnavailable");
    }

    #[tokio::test]
    async fn discovery_probe_input_accepts_valid_display_and_value() {
        let app = app_with_discovery(MockDiscovery::available_with_probe_current(
            vec!["K@P:d0e5:0"],
            Some(4626),
        ));

        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["accepted"], true);
        assert_eq!(json["displayId"], "K@P:d0e5:0");
        assert_eq!(json["value"], 4626);
        assert_eq!(json["current"], 4626);
    }

    #[tokio::test]
    async fn discovery_probe_input_works_without_config() {
        let app = router(Arc::new(AppState::with_discovery(
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "deskmux.config.json").into()),
            Box::new(MockDiscovery::available_with_probe_current(
                vec!["K@P:d0e5:0"],
                None,
            )),
        )));

        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["accepted"], true);
    }

    #[tokio::test]
    async fn discovery_probe_input_invalid_json_returns_400() {
        let app = app_with_discovery(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626));
        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":"4626"}"#,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["code"], "badRequest");
    }

    #[tokio::test]
    async fn discovery_probe_input_out_of_range_value_returns_400() {
        let app = app_with_discovery(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626));
        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":70000}"#,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["code"], "badRequest");
    }

    #[tokio::test]
    async fn discovery_probe_input_missing_display_is_404() {
        let app = app_with_discovery(MockDiscovery::available(vec!["K@P:d0e5:0"], READING_4626));
        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/GHOST%3A0000%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["code"], "displayNotFound");
    }

    #[tokio::test]
    async fn discovery_probe_input_unavailable_platform_is_501() {
        let app = app_with_discovery(MockDiscovery::unavailable());
        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(json["code"], "nativeUnavailable");
    }

    #[tokio::test]
    async fn discovery_probe_input_write_failure_is_500_with_code() {
        let app = app_with_discovery(MockDiscovery::failing_probe(
            vec!["K@P:d0e5:0"],
            DiscoveryError::VcpWriteFailed {
                detail: "monitor rejected write".to_string(),
            },
        ));
        let (status, json) = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["code"], "vcpWriteFailed");
    }

    #[tokio::test]
    async fn discovery_probe_input_records_success_and_failure_events() {
        let app = app_with_discovery(MockDiscovery::failing_probe(
            vec!["K@P:d0e5:0"],
            DiscoveryError::VcpWriteFailed {
                detail: "monitor rejected write".to_string(),
            },
        ));
        let _ = post_json(
            &app,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;
        let (_, failure_events) = get(&app, "/events").await;
        assert!(failure_events["events"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Test switch failed"));

        let app_success = app_with_discovery(MockDiscovery::available_with_probe_current(
            vec!["K@P:d0e5:0"],
            Some(4626),
        ));
        let _ = post_json(
            &app_success,
            "/native-ddc/displays/K%40P%3Ad0e5%3A0/probe-input",
            r#"{"value":4626}"#,
        )
        .await;
        let (_, success_events) = get(&app_success, "/events").await;
        assert!(success_events["events"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Test switch accepted"));
    }

    async fn get_with_origin(
        app: &Router,
        uri: &str,
        origin: &str,
    ) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(header::ORIGIN, origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, headers, json)
    }

    async fn options_preflight(
        app: &Router,
        uri: &str,
        origin: &str,
    ) -> (StatusCode, axum::http::HeaderMap) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri(uri)
                    .header(header::ORIGIN, origin)
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        (response.status(), response.headers().clone())
    }

    #[tokio::test]
    async fn cors_allows_localhost_dev_origin_on_status() {
        let app = app_with_config();
        let (status, headers, _) = get_with_origin(&app, "/status", "http://127.0.0.1:1430").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://127.0.0.1:1430"))
        );
    }

    #[tokio::test]
    async fn cors_allows_localhost_hostname_on_status() {
        let app = app_with_config();
        let (status, headers, _) = get_with_origin(&app, "/status", "http://localhost:1430").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://localhost:1430"))
        );
    }

    #[tokio::test]
    async fn cors_rejects_untrusted_origin() {
        let app = app_with_config();
        let (status, headers, _) = get_with_origin(&app, "/status", "http://evil.example").await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }

    #[tokio::test]
    async fn cors_options_preflight_for_apply_preset() {
        let app = app_with_config();
        let (status, headers) =
            options_preflight(&app, "/apply-preset", "http://127.0.0.1:1430").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://127.0.0.1:1430"))
        );
        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).is_some());
    }

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

    async fn get(app: &Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    async fn post_json(app: &Router, uri: &str, body: &str) -> (StatusCode, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn health_without_config_includes_config_error() {
        let app = app_without_config();
        let (status, json) = get(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["configLoaded"], false);
        let config_error = json["configError"]
            .as_str()
            .expect("configError should be present");
        assert!(config_error.contains("failed to read config file"));
    }

    #[tokio::test]
    async fn health_with_config_omits_config_error() {
        let app = app_with_config();
        let (status, json) = get(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["configLoaded"], true);
        assert!(json.get("configError").is_none());
    }

    #[tokio::test]
    async fn events_returns_recent_history() {
        let app = app_with_config();
        let (status, json) = get(&app, "/events").await;
        assert_eq!(status, StatusCode::OK);
        let events = json["events"].as_array().expect("events array");
        assert!(!events.is_empty());
        assert!(events[0]["message"]
            .as_str()
            .unwrap()
            .contains("Config loaded"));
    }

    #[tokio::test]
    async fn status_returns_503_when_config_not_loaded() {
        let app = app_without_config();
        let (status, json) = get(&app, "/status").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["error"], "config not loaded");
        assert!(json["configError"]
            .as_str()
            .unwrap()
            .contains("failed to read config file"));
    }

    #[tokio::test]
    async fn apply_preset_503_includes_config_error() {
        let app = app_without_config();
        let (status, json) =
            post_json(&app, "/apply-preset", r#"{"preset":"all_a","dryRun":true}"#).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["error"], "config not loaded");
        assert!(json["configError"]
            .as_str()
            .unwrap()
            .contains("failed to read config file"));
    }

    #[tokio::test]
    async fn status_returns_safe_summaries_without_shell_commands() {
        let app = app_with_config();
        let (status, json) = get(&app, "/status").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["deviceName"], "device-a");
        assert_eq!(json["presets"][0]["name"], "all_a");
        assert_eq!(json["monitors"][0]["id"], "monitor1");
        assert!(json.get("lastAppliedPreset").is_some());
        assert!(!json.to_string().contains("exit 0"));
        assert!(json.get("command").is_none());
        assert!(json.get("inputs").is_none());
    }

    #[tokio::test]
    async fn apply_preset_dry_run_returns_results_without_executing() {
        let app = app_with_config();
        let (status, json) =
            post_json(&app, "/apply-preset", r#"{"preset":"all_a","dryRun":true}"#).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["dryRun"], true);
        assert_eq!(json["localResults"][0]["outcome"]["type"], "dryRun");
        assert!(json["planningErrors"].as_array().unwrap().is_empty());
        assert!(json["peerResults"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn apply_preset_unknown_preset_returns_404() {
        let app = app_with_config();
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":"missing"}"#).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json["error"].as_str().unwrap().contains("missing"));
    }

    #[tokio::test]
    async fn apply_preset_empty_name_returns_400() {
        let app = app_with_config();
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":"  "}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "preset name is required");
    }

    #[tokio::test]
    async fn apply_preset_malformed_json_returns_400() {
        let app = app_with_config();
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "invalid JSON body");
    }

    #[tokio::test]
    async fn apply_preset_updates_last_applied_preset_on_status() {
        let app = app_with_config();
        let (_, apply_json) = post_json(
            &app,
            "/apply-preset",
            r#"{"preset":"all_a","dryRun":false,"localOnly":true}"#,
        )
        .await;
        assert_eq!(apply_json["dryRun"], false);
        assert_eq!(apply_json["localResults"][0]["outcome"]["type"], "success");

        let (_, status_json) = get(&app, "/status").await;
        assert_eq!(status_json["lastAppliedPreset"], "all_a");
    }

    #[tokio::test]
    async fn apply_preset_failure_does_not_update_last_applied_preset() {
        let mut config = test_config();
        config.monitors[0]
            .inputs
            .get_mut("device-a")
            .unwrap()
            .command = Some("exit 1".to_string());
        let app = router(Arc::new(AppState::from_load_result(Ok(config))));
        post_json(
            &app,
            "/apply-preset",
            r#"{"preset":"all_a","dryRun":false,"localOnly":true}"#,
        )
        .await;

        let (_, status_json) = get(&app, "/status").await;
        assert!(status_json["lastAppliedPreset"].is_null());
    }

    #[tokio::test]
    async fn apply_preset_dry_run_does_not_update_last_applied_preset() {
        let app = app_with_config();
        post_json(&app, "/apply-preset", r#"{"preset":"all_a","dryRun":true}"#).await;

        let (_, status_json) = get(&app, "/status").await;
        assert!(status_json["lastAppliedPreset"].is_null());
    }
}
