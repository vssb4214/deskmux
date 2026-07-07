use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};

use crate::config::{format_config_load_error, Config, LoadError};

use super::apply::{apply_preset_to_state, ApplyPresetStateError};
use super::discovery::{DiscoverySource, NativeDiscoverySource};
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
    pub discovery: Box<dyn DiscoverySource>,
    /// Per-display set of VCP 0x60 values a read has actually returned this session. The probe
    /// gate (`api::discovery::probe_input_gated`) only allows writing values present here — a
    /// probe can replay a value we know is real for this display, never a blind guess. Populated
    /// exclusively by successful reads (`read_input_source_handler`); shared with the Tauri
    /// `probe_input` command via the same `Arc<AppState>` the HTTP server uses, so there is one
    /// source of truth regardless of which surface a read or probe comes through.
    observed_input_values: Mutex<HashMap<String, HashSet<u32>>>,
}

impl AppState {
    pub fn from_load_result(result: Result<Config, LoadError>) -> Self {
        Self::from_load_result_at(&crate::config::default_config_path(), result)
    }

    pub fn from_load_result_at(path: &std::path::Path, result: Result<Config, LoadError>) -> Self {
        Self::with_discovery_at(path, result, Box::new(NativeDiscoverySource))
    }

    /// Like `from_load_result` but with an injected discovery source, so handler tests are
    /// deterministic on every platform instead of depending on real display hardware.
    pub fn with_discovery(
        result: Result<Config, LoadError>,
        discovery: Box<dyn DiscoverySource>,
    ) -> Self {
        Self::with_discovery_at(&crate::config::default_config_path(), result, discovery)
    }

    pub fn with_discovery_at(
        path: &std::path::Path,
        result: Result<Config, LoadError>,
        discovery: Box<dyn DiscoverySource>,
    ) -> Self {
        let events = Mutex::new(EventLog::new());
        match result {
            Ok(config) => {
                record_config_loaded(&events, &config.device_name);
                Self {
                    config: Some(config),
                    config_error: None,
                    last_applied_preset: Mutex::new(None),
                    events,
                    discovery,
                    observed_input_values: Mutex::new(HashMap::new()),
                }
            }
            Err(err) => {
                let detail = format_config_load_error(path, &err);
                record_config_error(&events, &detail);
                Self {
                    config: None,
                    config_error: Some(detail),
                    last_applied_preset: Mutex::new(None),
                    events,
                    discovery,
                    observed_input_values: Mutex::new(HashMap::new()),
                }
            }
        }
    }

    /// Records `value` as a real state `display_id` was observed in via a successful VCP read.
    /// Called only from the discovery read path — never from probe itself, so a probe can't
    /// bootstrap its own permission by "observing" the value it just wrote.
    pub fn record_observed_input_value(&self, display_id: &str, value: u32) {
        let mut observed = self
            .observed_input_values
            .lock()
            .expect("observed_input_values lock poisoned");
        observed
            .entry(display_id.to_string())
            .or_default()
            .insert(value);
    }

    /// Whether a read has ever returned `value` as the current input for `display_id` this
    /// session. The probe gate's sole authorization check.
    pub fn is_observed_input_value(&self, display_id: &str, value: u32) -> bool {
        let observed = self
            .observed_input_values
            .lock()
            .expect("observed_input_values lock poisoned");
        observed
            .get(display_id)
            .is_some_and(|values| values.contains(&value))
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
