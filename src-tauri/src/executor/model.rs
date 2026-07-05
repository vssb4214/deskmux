use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    PresetNotFound { preset_name: String },
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutorError::PresetNotFound { preset_name } => {
                write!(f, "preset '{preset_name}' does not exist")
            }
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Why a single layout entry (monitorId -> deviceId) couldn't be resolved to a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ResolutionError {
    UnknownMonitor {
        monitor_id: String,
    },
    UnknownDevice {
        monitor_id: String,
        device_id: String,
    },
    /// The input exists but has no usable backend right now — e.g. no shell `command`, and
    /// either no `nativeDdc` configured or native DDC isn't available on this build.
    NoBackendAvailable {
        monitor_id: String,
        device_id: String,
    },
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolutionError::UnknownMonitor { monitor_id } => {
                write!(f, "preset routes to unknown monitor '{monitor_id}'")
            }
            ResolutionError::UnknownDevice {
                monitor_id,
                device_id,
            } => write!(
                f,
                "monitor '{monitor_id}' has no configured command for device '{device_id}'"
            ),
            ResolutionError::NoBackendAvailable {
                monitor_id,
                device_id,
            } => write!(
                f,
                "monitor '{monitor_id}' has no usable backend for device '{device_id}'"
            ),
        }
    }
}

impl std::error::Error for ResolutionError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorResult {
    pub monitor_id: String,
    pub device_id: String,
    /// The resolved command, if resolution succeeded (whether or not it was executed).
    pub command: Option<String>,
    /// True only if a process was actually spawned.
    pub executed: bool,
    pub outcome: MonitorOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MonitorOutcome {
    /// Resolved successfully but not executed, because dry-run was requested.
    DryRun,
    Success {
        stdout: String,
        stderr: String,
    },
    /// Ran, but exited non-zero.
    Failed {
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
    },
    /// The process never started (e.g. the command couldn't be spawned).
    SpawnFailed {
        message: String,
    },
    ResolutionFailed {
        error: ResolutionError,
    },
}
