pub mod bind;
pub mod client;
mod handlers;
#[cfg(test)]
mod peer_orchestration;
mod server;
#[cfg(test)]
mod test_server;
pub mod types;

pub use bind::{resolve_bind_addr, DEFAULT_BIND_HOST, DEFAULT_PORT};
pub use client::{PeerClient, PeerClientError};
pub use handlers::AppState;
pub use server::spawn_server;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/apply-preset", post(handlers::apply_preset_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use tower::ServiceExt;

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
    async fn health_returns_ok_and_config_loaded_flag() {
        let app = router(AppState::new(None));
        let (status, json) = get(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["configLoaded"], false);
    }

    #[tokio::test]
    async fn status_returns_503_when_config_not_loaded() {
        let app = router(AppState::new(None));
        let (status, json) = get(&app, "/status").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["error"], "config not loaded");
    }

    #[tokio::test]
    async fn status_returns_safe_summaries_without_shell_commands() {
        let app = router(AppState::new(Some(test_config())));
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
        let app = router(AppState::new(Some(test_config())));
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
        let app = router(AppState::new(Some(test_config())));
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":"missing"}"#).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json["error"].as_str().unwrap().contains("missing"));
    }

    #[tokio::test]
    async fn apply_preset_empty_name_returns_400() {
        let app = router(AppState::new(Some(test_config())));
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":"  "}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "preset name is required");
    }

    #[tokio::test]
    async fn apply_preset_malformed_json_returns_400() {
        let app = router(AppState::new(Some(test_config())));
        let (status, json) = post_json(&app, "/apply-preset", r#"{"preset":}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "invalid JSON body");
    }

    #[tokio::test]
    async fn apply_preset_updates_last_applied_preset_on_status() {
        let app = router(AppState::new(Some(test_config())));
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
            .command = "exit 1".to_string();
        let app = router(AppState::new(Some(config)));
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
        let app = router(AppState::new(Some(test_config())));
        post_json(&app, "/apply-preset", r#"{"preset":"all_a","dryRun":true}"#).await;

        let (_, status_json) = get(&app, "/status").await;
        assert!(status_json["lastAppliedPreset"].is_null());
    }
}
