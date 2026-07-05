use std::sync::{Arc, Mutex};

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};

use crate::config::{Config, LoadError};

use super::apply::{apply_preset_to_state, ApplyPresetStateError};
use super::events::{record_config_error, record_config_loaded, EventLog};
use super::types::{
    ApplyPresetRequest, ApplyPresetResponse, ErrorResponse, EventsResponse, HealthResponse,
    MonitorSummary, PresetSummary, StatusResponse,
};

pub struct AppState {
    pub config: Option<Config>,
    pub config_error: Option<String>,
    pub last_applied_preset: Mutex<Option<String>>,
    pub events: Mutex<EventLog>,
}

impl AppState {
    pub fn from_load_result(result: Result<Config, LoadError>) -> Self {
        let events = Mutex::new(EventLog::new());
        match result {
            Ok(config) => {
                record_config_loaded(&events, &config.device_name);
                Self {
                    config: Some(config),
                    config_error: None,
                    last_applied_preset: Mutex::new(None),
                    events,
                }
            }
            Err(err) => {
                let detail = err.to_string();
                record_config_error(&events, &detail);
                Self {
                    config: None,
                    config_error: Some(detail),
                    last_applied_preset: Mutex::new(None),
                    events,
                }
            }
        }
    }
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        config_loaded: state.config.is_some(),
        config_error: state.config_error.clone(),
    })
}

pub async fn status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let config = state
        .config
        .as_ref()
        .ok_or_else(|| config_not_loaded(&state))?;

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

pub async fn events(State(state): State<Arc<AppState>>) -> Json<EventsResponse> {
    let log = state.events.lock().expect("event log lock poisoned");
    Json(EventsResponse {
        events: log.recent(super::events::MAX_EVENTS),
    })
}

pub async fn apply_preset_handler(
    State(state): State<Arc<AppState>>,
    body: Result<Json<ApplyPresetRequest>, JsonRejection>,
) -> Result<Json<ApplyPresetResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Json(body) = body.map_err(|_| bad_request("invalid JSON body"))?;

    if body.preset.trim().is_empty() {
        return Err(bad_request("preset name is required"));
    }

    state
        .config
        .as_ref()
        .ok_or_else(|| config_not_loaded(&state))?;

    match apply_preset_to_state(
        &state,
        &body.preset,
        body.dry_run,
        body.local_only,
        super::events::ApplySource::Api,
    )
    .await
    {
        Ok(result) => Ok(Json(ApplyPresetResponse {
            preset: result.preset,
            dry_run: result.dry_run,
            local_only: result.local_only,
            planning_errors: result.planning_errors,
            local_results: result.local_results,
            peer_results: result.peer_results,
        })),
        Err(ApplyPresetStateError::ConfigNotLoaded) => Err(config_not_loaded(&state)),
        Err(ApplyPresetStateError::PresetNotFound { preset_name }) => {
            Err(not_found(format!("preset '{preset_name}' does not exist")))
        }
    }
}

fn config_not_loaded(state: &AppState) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "config not loaded".to_string(),
            config_error: state.config_error.clone(),
        }),
    )
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
            config_error: None,
        }),
    )
}

fn not_found(message: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message,
            config_error: None,
        }),
    )
}
