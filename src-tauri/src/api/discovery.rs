//! HTTP surface for in-app native DDC discovery (see docs/NATIVE_DDC_DISCOVERY.md). Works
//! without a loaded config — first-run means no config exists yet, which is exactly when
//! discovery matters.
//!
//! Probe (test-switch) writes are deliberately NOT exposed here. Discovery reads are safe on
//! any local process that can reach the loopback API; a probe write is not, so it's reachable
//! only via the Tauri `probe_input` IPC command (`commands.rs`) — invokable solely from the
//! bundled webview, never plain HTTP. `probe_input_gated` below is that command's business
//! logic, kept in this module because it reuses `DiscoverySource` and `AppState` directly.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::events::record_probe_input_result;
use crate::executor::discovery::{
    self, ControlReading, ControlWriteResult, DiscoveredDisplay, DiscoveryError, InputSourceReading,
};
use crate::executor::NativeDdcFeature;

use super::handlers::AppState;
use super::types::{
    DiscoveryDisplaySummary, DiscoveryDisplaysResponse, DiscoveryErrorResponse,
    InputSourceResponse, NativeDdcControlState, NativeDdcControls, NativeDdcControlsResponse,
    ProbeInputResponse, SetNativeDdcControlResponse,
};

/// Where discovery results come from, behind a trait so handler tests inject scripted sources
/// and stay deterministic on every CI platform — the real source is platform-dependent.
pub trait DiscoverySource: Send + Sync {
    fn native_available(&self) -> bool;
    fn list_displays(&self) -> Result<Vec<DiscoveredDisplay>, DiscoveryError>;
    fn read_input_source(&self, display_id: &str) -> Result<InputSourceReading, DiscoveryError>;
    fn read_control(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
    ) -> Result<ControlReading, DiscoveryError>;
    fn set_control(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
        value: u16,
    ) -> Result<ControlWriteResult, DiscoveryError>;
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

    fn read_control(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
    ) -> Result<ControlReading, DiscoveryError> {
        discovery::read_control(display_id, feature)
    }

    fn set_control(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
        value: u16,
    ) -> Result<ControlWriteResult, DiscoveryError> {
        discovery::set_control(display_id, feature, value)
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
    let display_id_for_record = display_id.clone();
    let state_for_read = state.clone();

    let result = tokio::task::spawn_blocking(move || {
        state_for_read.discovery.read_input_source(&display_id)
    })
    .await
    .map_err(join_error)?;

    result
        .map(|reading| {
            // Only a successful read ever authorizes a probe of this value — see
            // AppState::record_observed_input_value.
            state.record_observed_input_value(&display_id_for_record, reading.current);
            Json(InputSourceResponse {
                current: reading.current,
                maximum: reading.maximum,
            })
        })
        .map_err(discovery_error)
}

pub async fn read_controls_handler(
    State(state): State<Arc<AppState>>,
    Path(display_id): Path<String>,
) -> Result<Json<NativeDdcControlsResponse>, (StatusCode, Json<DiscoveryErrorResponse>)> {
    let result = tokio::task::spawn_blocking(move || read_controls_response(&state, &display_id))
        .await
        .map_err(join_error)?;

    result.map(Json).map_err(discovery_error)
}

fn read_controls_response(
    state: &AppState,
    display_id: &str,
) -> Result<NativeDdcControlsResponse, DiscoveryError> {
    if !state.discovery.native_available() {
        return Ok(NativeDdcControlsResponse {
            display_id: display_id.to_string(),
            controls: NativeDdcControls {
                brightness: unavailable_control("nativeUnavailable"),
                contrast: unavailable_control("nativeUnavailable"),
                volume: unavailable_control("nativeUnavailable"),
            },
        });
    }

    Ok(NativeDdcControlsResponse {
        display_id: display_id.to_string(),
        controls: NativeDdcControls {
            brightness: read_control_state(state, display_id, NativeDdcFeature::Brightness)?,
            contrast: read_control_state(state, display_id, NativeDdcFeature::Contrast)?,
            volume: read_control_state(state, display_id, NativeDdcFeature::Volume)?,
        },
    })
}

fn read_control_state(
    state: &AppState,
    display_id: &str,
    feature: NativeDdcFeature,
) -> Result<NativeDdcControlState, DiscoveryError> {
    match state.discovery.read_control(display_id, feature) {
        Ok(reading) => Ok(NativeDdcControlState {
            available: true,
            current: Some(reading.current),
            maximum: Some(reading.maximum),
            error: None,
        }),
        Err(DiscoveryError::DisplayNotFound { display_id }) => {
            Err(DiscoveryError::DisplayNotFound { display_id })
        }
        Err(err) => Ok(unavailable_control(err.code())),
    }
}

fn unavailable_control(error: &str) -> NativeDdcControlState {
    NativeDdcControlState {
        available: false,
        current: None,
        maximum: None,
        error: Some(error.to_string()),
    }
}

/// Business logic for the Tauri `probe_input` command (see `commands.rs`). Writes only if
/// `value` has previously been observed as a real `current` reading for `display_id` — enforced
/// here, not just in whatever UI calls this, so the check can't be bypassed by invoking the
/// command directly with an untested value.
pub fn probe_input_gated(
    state: &AppState,
    display_id: &str,
    value: u16,
) -> Result<ProbeInputResponse, DiscoveryErrorResponse> {
    if !state.is_observed_input_value(display_id, u32::from(value)) {
        let message = format!(
            "value {value} has not been read as the current input on display '{display_id}' \
             this session — read this display's current input before probing a value"
        );
        record_probe_input_result(
            &state.events,
            display_id,
            value,
            false,
            Some(message.clone()),
        );
        return Err(DiscoveryErrorResponse {
            error: message,
            code: "valueNotObserved".to_string(),
        });
    }

    match state.discovery.probe_input(display_id, value) {
        Ok(probe) => {
            record_probe_input_result(&state.events, display_id, value, true, None);
            Ok(ProbeInputResponse {
                accepted: probe.accepted,
                display_id: display_id.to_string(),
                value,
                current: probe.current,
            })
        }
        Err(err) => {
            record_probe_input_result(
                &state.events,
                display_id,
                value,
                false,
                Some(err.to_string()),
            );
            Err(DiscoveryErrorResponse {
                error: err.to_string(),
                code: err.code().to_string(),
            })
        }
    }
}

pub fn set_native_ddc_control_gated(
    state: &AppState,
    display_id: &str,
    feature_name: &str,
    value: i64,
) -> Result<SetNativeDdcControlResponse, DiscoveryErrorResponse> {
    let feature = parse_live_control_feature(feature_name)?;
    if !(0..=i64::from(u16::MAX)).contains(&value) {
        return Err(DiscoveryErrorResponse {
            error: format!(
                "value {value} is outside the supported range 0..={}",
                u16::MAX
            ),
            code: "invalidControlValue".to_string(),
        });
    }
    let value = value as u16;

    match state.discovery.set_control(display_id, feature, value) {
        Ok(result) => {
            super::events::record_native_ddc_control_result(
                &state.events,
                display_id,
                feature.label(),
                value,
                true,
                None,
            );
            Ok(SetNativeDdcControlResponse {
                accepted: result.accepted,
                display_id: display_id.to_string(),
                feature: feature.api_name().to_string(),
                value: result.value,
                maximum: result.maximum,
            })
        }
        Err(err) => {
            super::events::record_native_ddc_control_result(
                &state.events,
                display_id,
                feature.label(),
                value,
                false,
                Some(err.to_string()),
            );
            Err(DiscoveryErrorResponse {
                error: err.to_string(),
                code: err.code().to_string(),
            })
        }
    }
}

fn parse_live_control_feature(feature: &str) -> Result<NativeDdcFeature, DiscoveryErrorResponse> {
    match feature {
        "brightness" => Ok(NativeDdcFeature::Brightness),
        "contrast" => Ok(NativeDdcFeature::Contrast),
        "volume" => Ok(NativeDdcFeature::Volume),
        _ => Err(DiscoveryErrorResponse {
            error: format!(
                "unsupported native DDC control '{feature}'; expected brightness, contrast, or volume"
            ),
            code: "unsupportedFeature".to_string(),
        }),
    }
}

fn discovery_error(err: DiscoveryError) -> (StatusCode, Json<DiscoveryErrorResponse>) {
    let status = match &err {
        DiscoveryError::DisplayNotFound { .. } => StatusCode::NOT_FOUND,
        DiscoveryError::NativeUnavailable => StatusCode::NOT_IMPLEMENTED,
        DiscoveryError::EnumerationFailed { .. }
        | DiscoveryError::VcpReadFailed { .. }
        | DiscoveryError::VcpWriteFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        DiscoveryError::InvalidControlValue { .. } => StatusCode::BAD_REQUEST,
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
