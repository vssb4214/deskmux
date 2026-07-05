use async_trait::async_trait;

use crate::api::PeerClientError;
use crate::executor::MonitorResult;

/// Response from a peer's local-only preset apply (transport layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerApplyResponse {
    pub local_results: Vec<MonitorResult>,
}

#[async_trait]
pub trait PeerApplyClient: Send + Sync {
    async fn apply_preset_local(
        &self,
        host: &str,
        port: u16,
        preset: &str,
        dry_run: bool,
    ) -> Result<PeerApplyResponse, PeerClientError>;
}

pub struct PeerClientAdapter;

#[async_trait]
impl PeerApplyClient for PeerClientAdapter {
    async fn apply_preset_local(
        &self,
        host: &str,
        port: u16,
        preset: &str,
        dry_run: bool,
    ) -> Result<PeerApplyResponse, PeerClientError> {
        let client = crate::api::PeerClient::new(host, port);
        let response = client.apply_preset(preset, dry_run, true).await?;
        Ok(PeerApplyResponse {
            local_results: response.local_results,
        })
    }
}
