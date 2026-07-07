//! HTTP surface for in-app native DDC discovery and setup-time probe writes (see
//! docs/NATIVE_DDC_DISCOVERY.md). Works without a loaded config — first-run means no config
//! exists yet, which is exactly when discovery and test switching matter.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};

use crate::api::events::record_probe_input_result;
use crate::executor::discovery::{self, DiscoveredDisplay, DiscoveryError, InputSourceReading};

use super::handlers::AppState;
use super::types::{
    DiscoveryDisplaySummary, DiscoveryDisplaysResponse, DiscoveryErrorResponse,
    InputSourceResponse, ProbeInputRequest, ProbeInputResponse,
};

/// Where discovery results come from, behind a trait so handler tests inject scripted sources
/// and stay deterministic on every CI platform — the real source is platform-dependent.
pub trait DiscoverySource: Send + Sync {
    fn native_available(&self) -> bool;
    fn list_displays(&self) -> Result<Vec<DiscoveredDisplay>, DiscoveryError>;
    fn read_input_source(&self, display_id: &str) -> Result<InputSourceReading, DiscoveryError>;
    fn probe_input(
        &self,
        display_id: &str,
        value: u16,
    ) -> Result<discovery::ProbeInputResult, DiscoveryError>;
}

/// Production source: delegates to `executor::discovery` (real Windows DDC calls there;
/// honest unavailability elsewhere).
pub struct NativeDiscoverySource;

impl DiscoverySource for NativeDiscoverySource {
    fn native_available(&self) -> bool {
        discovery::native_available()
    }

    fn list_displays(&self) -> Result<Vec<DiscoveredDisplay>, DiscoveryError> {
        discovery::list_displays()
    }

    fn read_input_source(&self, display_id: &str) -> Result<InputSourceReading, DiscoveryError> {
        discovery::read_input_source(display_id)
    }

    fn probe_input(
        &self,
        display_id: &str,
        value: u16,
    ) -> Result<discovery::ProbeInputResult, DiscoveryError> {
        discovery::probe_input(display_id, value)
    }
}

pub async fn list_displays_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DiscoveryDisplaysResponse>, (StatusCode, Json<DiscoveryErrorResponse>)> {
    // DDC enumeration blocks on Windows API calls (and the read path may retry); keep it off
    // the async workers.
    let result = tokio::task::spawn_blocking(move || {
        let native_available = state.discovery.native_available();
        state
            .discovery
            .list_displays()
            .map(|displays| DiscoveryDisplaysResponse {
                native_available,
                displays: displays
                    .into_iter()
                    .map(|d| DiscoveryDisplaySummary {
                        display_id: d.display_id,
                    })
                    .collect(),
            })
    })
    .await
    .map_err(join_error)?;

    result.map(Json).map_err(discovery_error)
}

pub async fn read_input_source_handler(
    State(state): State<Arc<AppState>>,
    Path(display_id): Path<String>,
) -> Result<Json<InputSourceResponse>, (StatusCode, Json<DiscoveryErrorResponse>)> {
    let result =
        tokio::task::spawn_blocking(move || state.discovery.read_input_source(&display_id))
            .await
            .map_err(join_error)?;

    result
        .map(|reading| {
            Json(InputSourceResponse {
                current: reading.current,
                maximum: reading.maximum,
            })
        })
        .map_err(discovery_error)
}

pub async fn probe_input_handler(
    State(state): State<Arc<AppState>>,
    Path(display_id): Path<String>,
    body: Result<Json<ProbeInputRequest>, JsonRejection>,
) -> Result<Json<ProbeInputResponse>, (StatusCode, Json<DiscoveryErrorResponse>)> {
    let Json(body) = body.map_err(|_| bad_request("invalid JSON body"))?;
    let value = body.value;
    let display_id_for_event = display_id.clone();
    let state_for_probe = state.clone();

    let result = tokio::task::spawn_blocking(move || {
        state_for_probe.discovery.probe_input(&display_id, value)
    })
    .await
    .map_err(join_error)?;

    match result {
        Ok(probe) => {
            record_probe_input_result(&state.events, &display_id_for_event, value, true, None);
            Ok(Json(ProbeInputResponse {
                accepted: probe.accepted,
                display_id: display_id_for_event,
                value,
                current: probe.current,
            }))
        }
        Err(err) => {
            record_probe_input_result(
                &state.events,
                &display_id_for_event,
                value,
                false,
                Some(err.to_string()),
            );
            Err(discovery_error(err))
        }
    }
}

fn discovery_error(err: DiscoveryError) -> (StatusCode, Json<DiscoveryErrorResponse>) {
    let status = match &err {
        DiscoveryError::DisplayNotFound { .. } => StatusCode::NOT_FOUND,
        DiscoveryError::NativeUnavailable => StatusCode::NOT_IMPLEMENTED,
        DiscoveryError::EnumerationFailed { .. }
        | DiscoveryError::VcpReadFailed { .. }
        | DiscoveryError::VcpWriteFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(DiscoveryErrorResponse {
            error: err.to_string(),
            code: err.code().to_string(),
        }),
    )
}

fn join_error(err: tokio::task::JoinError) -> (StatusCode, Json<DiscoveryErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(DiscoveryErrorResponse {
            error: format!("discovery task failed: {err}"),
            code: "internal".to_string(),
        }),
    )
}

fn bad_request(message: &str) -> (StatusCode, Json<DiscoveryErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(DiscoveryErrorResponse {
            error: message.to_string(),
            code: "badRequest".to_string(),
        }),
    )
}
