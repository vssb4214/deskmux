use serde::{Deserialize, Serialize};

use crate::executor::MonitorResult;
pub use crate::orchestrator::{PeerApplyOutcome, PlanningError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub config_loaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSummary {
    pub name: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSummary {
    pub id: String,
    pub label: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub device_name: String,
    pub presets: Vec<PresetSummary>,
    pub monitors: Vec<MonitorSummary>,
    pub last_applied_preset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPresetRequest {
    pub preset: String,
    #[serde(default)]
    pub dry_run: bool,
    /// When true, only run monitors owned by this machine — no peer fan-out.
    #[serde(default)]
    pub local_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPresetResponse {
    pub preset: String,
    pub dry_run: bool,
    pub local_only: bool,
    pub planning_errors: Vec<PlanningError>,
    pub local_results: Vec<MonitorResult>,
    pub peer_results: Vec<PeerApplyOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub events: Vec<crate::api::events::DeskMuxEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_error: Option<String>,
}
