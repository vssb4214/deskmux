use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    DeviceNameNotFound {
        device_name: String,
    },
    DuplicateDeviceId {
        device_id: String,
    },
    DuplicateMonitorId {
        monitor_id: String,
    },
    UnknownDeviceInMonitorInput {
        monitor_id: String,
        device_id: String,
    },
    UnknownMonitorInPresetLayout {
        preset_name: String,
        monitor_id: String,
    },
    UnknownDeviceInPresetLayout {
        preset_name: String,
        monitor_id: String,
        device_id: String,
    },
    DeviceNotInputForMonitor {
        preset_name: String,
        monitor_id: String,
        device_id: String,
    },
    UnknownControlledBy {
        monitor_id: String,
        controlled_by: String,
    },
    LocallyOwnedMonitorMissingInputs {
        monitor_id: String,
    },
    PeerNameNotFound {
        peer_name: String,
    },
    PeerNameIsLocalDevice {
        peer_name: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::DeviceNameNotFound { device_name } => write!(
                f,
                "deviceName '{device_name}' does not match any entry in devices[]"
            ),
            ConfigError::DuplicateDeviceId { device_id } => {
                write!(f, "duplicate device id '{device_id}' in devices[]")
            }
            ConfigError::DuplicateMonitorId { monitor_id } => {
                write!(f, "duplicate monitor id '{monitor_id}' in monitors[]")
            }
            ConfigError::UnknownDeviceInMonitorInput {
                monitor_id,
                device_id,
            } => write!(
                f,
                "monitor '{monitor_id}' declares an input for unknown device '{device_id}'"
            ),
            ConfigError::UnknownMonitorInPresetLayout {
                preset_name,
                monitor_id,
            } => write!(
                f,
                "preset '{preset_name}' routes unknown monitor '{monitor_id}'"
            ),
            ConfigError::UnknownDeviceInPresetLayout {
                preset_name,
                monitor_id,
                device_id,
            } => write!(
                f,
                "preset '{preset_name}' routes monitor '{monitor_id}' to unknown device '{device_id}'"
            ),
            ConfigError::DeviceNotInputForMonitor {
                preset_name,
                monitor_id,
                device_id,
            } => write!(
                f,
                "preset '{preset_name}' routes monitor '{monitor_id}' to '{device_id}', but {monitor_id} has no input for that device"
            ),
            ConfigError::UnknownControlledBy {
                monitor_id,
                controlled_by,
            } => write!(
                f,
                "monitor '{monitor_id}' has unknown controlledBy '{controlled_by}' (must match a devices[].id)"
            ),
            ConfigError::LocallyOwnedMonitorMissingInputs { monitor_id } => write!(
                f,
                "monitor '{monitor_id}' is owned by this machine but declares no inputs"
            ),
            ConfigError::PeerNameNotFound { peer_name } => write!(
                f,
                "peer '{peer_name}' does not match any entry in devices[]"
            ),
            ConfigError::PeerNameIsLocalDevice { peer_name } => write!(
                f,
                "peer '{peer_name}' must not name this machine (deviceName)"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// A collection of every problem found in a config, so a user sees all of
/// them at once instead of fixing one and re-running to find the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigErrors(pub Vec<ConfigError>);

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = self.0.iter().map(|e| format!("  - {e}")).collect();
        write!(f, "{}", lines.join("\n"))
    }
}

impl std::error::Error for ConfigErrors {}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    Invalid(ConfigErrors),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "failed to read config file: {err}"),
            LoadError::Parse(err) => write!(f, "failed to parse config file: {err}"),
            LoadError::Invalid(errors) => write!(f, "config is invalid:\n{errors}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(err) => Some(err),
            LoadError::Parse(err) => Some(err),
            LoadError::Invalid(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        LoadError::Io(err)
    }
}

impl From<serde_json::Error> for LoadError {
    fn from(err: serde_json::Error) -> Self {
        LoadError::Parse(err)
    }
}

impl From<ConfigErrors> for LoadError {
    fn from(err: ConfigErrors) -> Self {
        LoadError::Invalid(err)
    }
}
