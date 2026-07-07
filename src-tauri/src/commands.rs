use std::sync::Arc;

use tauri::State;

use crate::api::bind::dashboard_api_base_url;
use crate::api::discovery::probe_input_gated;
use crate::api::types::{DiscoveryErrorResponse, ProbeInputResponse};
use crate::api::AppState;
use crate::config::{
    parse_config_draft, save_config_draft as persist_config_draft, LoadError, SaveConfigResult,
};
use crate::BootstrapState;

#[tauri::command]
pub fn get_api_base_url(state: State<BootstrapState>) -> String {
    state.api_base_url.clone()
}

/// Parse and validate a config draft JSON string. Side-effect free — does not write to disk.
#[tauri::command]
pub fn validate_config_draft(json: String) -> Result<(), LoadError> {
    parse_config_draft(&json).map(|_| ())
}

/// Validate and atomically save a config draft to the fixed `deskmux.config.json` path.
#[tauri::command]
pub fn save_config_draft(json: String) -> Result<SaveConfigResult, LoadError> {
    persist_config_draft(&json)
}

/// Setup-time test switch: writes `value` to VCP 0x60 on `display_id`. IPC-only, deliberately
/// not HTTP — see api/discovery.rs's module comment for why. `probe_input_gated` enforces that
/// `value` was previously observed via a real read for this display; nothing here can bypass
/// that check by calling with different arguments.
#[tauri::command]
pub fn probe_input(
    display_id: String,
    value: u16,
    state: State<Arc<AppState>>,
) -> Result<ProbeInputResponse, DiscoveryErrorResponse> {
    probe_input_gated(&state, &display_id, value)
}

pub fn api_base_url_from_config(config: Option<&crate::config::Config>) -> String {
    dashboard_api_base_url(config)
}
