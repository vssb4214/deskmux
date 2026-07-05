use std::sync::{Arc, Mutex};

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};

use crate::config::Config;
use crate::executor::ExecutorError;
use crate::orchestrator::{apply_preset, PeerClientAdapter};

use super::types::{
    ApplyPresetRequest, ApplyPresetResponse, ErrorResponse, HealthResponse, MonitorSummary,
    PresetSummary, StatusResponse,
};

pub struct AppState {
    pub config: Option<Config>,
    pub last_applied_preset: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(config: Option<Config>) -> Self {
        Self {
            config,
            last_applied_preset: Mutex::new(None),
        }
    }
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        config_loaded: state.config.is_some(),
    })
}

pub async fn status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let config = state
        .config
        .as_ref()
        .ok_or(service_unavailable("config not loaded"))?;

    let last_applied_preset = state
        .last_applied_preset
        .lock()
        .expect("last_applied_preset lock poisoned")
        .clone();

    let mut presets: Vec<PresetSummary> = config
        .presets
        .iter()
        .map(|(name, preset)| PresetSummary {
            name: name.clone(),
            label: preset.label.clone(),
        })
        .collect();
    presets.sort_by(|a, b| a.name.cmp(&b.name));

    let mut monitors: Vec<MonitorSummary> = config
        .monitors
        .iter()
        .map(|monitor| MonitorSummary {
            id: monitor.id.clone(),
            label: monitor.label.clone(),
            order: monitor.order,
        })
        .collect();
    monitors.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));

    Ok(Json(StatusResponse {
        device_name: config.device_name.clone(),
        presets,
        monitors,
        last_applied_preset,
    }))
}

pub async fn apply_preset_handler(
    State(state): State<Arc<AppState>>,
    body: Result<Json<ApplyPresetRequest>, JsonRejection>,
) -> Result<Json<ApplyPresetResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Json(body) = body.map_err(|_| bad_request("invalid JSON body"))?;

    if body.preset.trim().is_empty() {
        return Err(bad_request("preset name is required"));
    }

    let config = state
        .config
        .as_ref()
        .ok_or(service_unavailable("config not loaded"))?;

    let peer_client = PeerClientAdapter;
    match apply_preset(
        config,
        &body.preset,
        body.dry_run,
        body.local_only,
        &peer_client,
    )
    .await
    {
        Ok(result) => {
            if !body.dry_run && result.is_full_success() {
                *state
                    .last_applied_preset
                    .lock()
                    .expect("last_applied_preset lock poisoned") = Some(body.preset.clone());
            }
            Ok(Json(ApplyPresetResponse {
                preset: result.preset,
                dry_run: result.dry_run,
                local_only: result.local_only,
                planning_errors: result.planning_errors,
                local_results: result.local_results,
                peer_results: result.peer_results,
            }))
        }
        Err(ExecutorError::PresetNotFound { preset_name }) => {
            Err(not_found(format!("preset '{preset_name}' does not exist")))
        }
    }
}

fn service_unavailable(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn not_found(message: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: message }),
    )
}
