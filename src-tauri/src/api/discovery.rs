//! HTTP surface for in-app monitor discovery (see docs/NATIVE_DDC_DISCOVERY.md). Read-only:
//! these endpoints never write to a monitor or to config. They work without a loaded config —
//! first-run means no config exists yet, which is exactly when discovery matters.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::executor::discovery::{self, DiscoveredDisplay, DiscoveryError, InputSourceReading};

use super::handlers::AppState;
use super::types::{
    DiscoveryDisplaySummary, DiscoveryDisplaysResponse, DiscoveryErrorResponse, InputSourceResponse,
};

/// Where discovery results come from, behind a trait so handler tests inject scripted sources
/// and stay deterministic on every CI platform — the real source is platform-dependent.
pub trait DiscoverySource: Send + Sync {
    fn native_available(&self) -> bool;
    fn list_displays(&self) -> Result<Vec<DiscoveredDisplay>, DiscoveryError>;
    fn read_input_source(&self, display_id: &str) -> Result<InputSourceReading, DiscoveryError>;
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

fn discovery_error(err: DiscoveryError) -> (StatusCode, Json<DiscoveryErrorResponse>) {
    let status = match &err {
        DiscoveryError::DisplayNotFound { .. } => StatusCode::NOT_FOUND,
        DiscoveryError::NativeUnavailable => StatusCode::NOT_IMPLEMENTED,
        DiscoveryError::EnumerationFailed { .. } | DiscoveryError::VcpReadFailed { .. } => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
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
