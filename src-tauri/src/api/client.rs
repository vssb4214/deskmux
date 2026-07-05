//! Thin typed HTTP client for calling a peer DeskMux API.

use std::fmt;

use reqwest::StatusCode as HttpStatus;

pub use super::types::{ApplyPresetRequest, ApplyPresetResponse, ErrorResponse, HealthResponse};

/// Thin typed HTTP client for calling a peer DeskMux API (`/health`, `/apply-preset` only).
pub struct PeerClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub enum PeerClientError {
    Request(reqwest::Error),
    Http { status: HttpStatus, error: String },
}

impl fmt::Display for PeerClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerClientError::Request(err) => write!(f, "peer request failed: {err}"),
            PeerClientError::Http { status, error } => {
                write!(f, "peer returned {status}: {error}")
            }
        }
    }
}

impl std::error::Error for PeerClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PeerClientError::Request(err) => Some(err),
            PeerClientError::Http { .. } => None,
        }
    }
}

impl PeerClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{host}:{port}"),
            http: reqwest::Client::new(),
        }
    }

    pub async fn health(&self) -> Result<HealthResponse, PeerClientError> {
        let url = format!("{}/health", self.base_url);
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(PeerClientError::Request)?;
        decode_json(response).await
    }

    pub async fn apply_preset(
        &self,
        preset: &str,
        dry_run: bool,
    ) -> Result<ApplyPresetResponse, PeerClientError> {
        let url = format!("{}/apply-preset", self.base_url);
        let body = ApplyPresetRequest {
            preset: preset.to_string(),
            dry_run,
        };
        let response = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(PeerClientError::Request)?;
        decode_json(response).await
    }
}

async fn decode_json<T>(response: reqwest::Response) -> Result<T, PeerClientError>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    if status.is_success() {
        return response.json::<T>().await.map_err(PeerClientError::Request);
    }

    let error = response
        .json::<ErrorResponse>()
        .await
        .map(|body| body.error)
        .unwrap_or_else(|_| status.to_string());

    Err(PeerClientError::Http { status, error })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{router, AppState};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    fn test_config() -> crate::config::Config {
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
                        "device-a": { "type": "hdmi", "command": "cmd-a" }
                    }
                }
            ],
            "presets": {
                "all_a": { "label": "All A", "layout": { "monitor1": "device-a" } }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    async fn spawn_test_server(state: AppState) -> (SocketAddr, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, router(state))
                .await
                .expect("serve test API");
        });
        (addr, handle)
    }

    #[tokio::test]
    async fn client_health_returns_config_loaded_flag() {
        let (addr, server) = spawn_test_server(AppState::new(Some(test_config()))).await;
        let client = PeerClient::new("127.0.0.1", addr.port());

        let health = client.health().await.expect("health should succeed");

        assert_eq!(health.status, "ok");
        assert!(health.config_loaded);
        server.abort();
    }

    #[tokio::test]
    async fn client_apply_preset_dry_run() {
        let (addr, server) = spawn_test_server(AppState::new(Some(test_config()))).await;
        let client = PeerClient::new("127.0.0.1", addr.port());

        let response = client
            .apply_preset("all_a", true)
            .await
            .expect("apply-preset should succeed");

        assert_eq!(response.preset, "all_a");
        assert!(response.dry_run);
        assert_eq!(response.results.len(), 1);
        server.abort();
    }

    #[tokio::test]
    async fn client_surfaces_http_errors() {
        let (addr, server) = spawn_test_server(AppState::new(Some(test_config()))).await;
        let client = PeerClient::new("127.0.0.1", addr.port());

        let err = client
            .apply_preset("missing", false)
            .await
            .expect_err("unknown preset should fail");

        assert!(matches!(err, PeerClientError::Http { .. }));
        if let PeerClientError::Http { status, error } = err {
            assert_eq!(status, HttpStatus::NOT_FOUND);
            assert!(error.contains("missing"));
        }
        server.abort();
    }
}
