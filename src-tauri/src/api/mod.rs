pub mod bind;
pub mod client;
mod cors;
mod handlers;
#[cfg(test)]
mod peer_orchestration;
mod server;
#[cfg(test)]
mod test_server;
pub mod types;

pub use bind::{dashboard_api_base_url, resolve_bind_addr, DEFAULT_BIND_HOST, DEFAULT_PORT};
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

    fn app_with_config() -> Router {
        router(AppState::from_load_result(Ok(test_config())))
    }

    fn app_without_config() -> Router {
        router(AppState::from_load_result(Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "deskmux.config.json",
        )
        .into())))
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
            .command = "exit 1".to_string();
        let app = router(AppState::from_load_result(Ok(config)));
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
