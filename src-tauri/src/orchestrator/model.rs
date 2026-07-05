use serde::{Deserialize, Serialize};

use crate::executor::{MonitorOutcome, MonitorResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningError {
    pub monitor_id: String,
    #[serde(rename = "type")]
    pub kind: PlanningErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanningErrorKind {
    UnknownMonitor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerRef {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerApplyOutcome {
    pub device_id: String,
    pub peer: Option<PeerRef>,
    pub outcome: PeerOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PeerOutcome {
    Success {
        local_only: bool,
        results: Vec<MonitorResult>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        peer_results: Vec<PeerApplyOutcome>,
    },
    Failed {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatedApplyResult {
    pub preset: String,
    pub dry_run: bool,
    pub local_only: bool,
    pub planning_errors: Vec<PlanningError>,
    pub local_results: Vec<MonitorResult>,
    pub peer_results: Vec<PeerApplyOutcome>,
}

impl CoordinatedApplyResult {
    pub fn is_full_success(&self) -> bool {
        if !self.planning_errors.is_empty() {
            return false;
        }
        if !self.local_results.iter().all(monitor_result_is_success) {
            return false;
        }
        self.peer_results.iter().all(peer_results_are_successful)
    }
}

fn monitor_result_is_success(result: &MonitorResult) -> bool {
    matches!(
        result.outcome,
        MonitorOutcome::DryRun | MonitorOutcome::Success { .. }
    )
}

fn peer_results_are_successful(peer: &PeerApplyOutcome) -> bool {
    match &peer.outcome {
        PeerOutcome::Success { results, .. } => results.iter().all(monitor_result_is_success),
        PeerOutcome::Failed { .. } => false,
    }
}
