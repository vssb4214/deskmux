use std::collections::{HashMap, HashSet};

use super::error::{ConfigError, ConfigErrors};
use super::model::{Config, Monitor};

pub fn validate(config: &Config) -> Result<(), ConfigErrors> {
    let mut errors = Vec::new();

    let mut device_ids: HashSet<&str> = HashSet::new();
    for device in &config.devices {
        if !device_ids.insert(device.id.as_str()) {
            errors.push(ConfigError::DuplicateDeviceId {
                device_id: device.id.clone(),
            });
        }
    }

    let mut monitor_ids: HashSet<&str> = HashSet::new();
    for monitor in &config.monitors {
        if !monitor_ids.insert(monitor.id.as_str()) {
            errors.push(ConfigError::DuplicateMonitorId {
                monitor_id: monitor.id.clone(),
            });
        }
    }

    if !device_ids.contains(config.device_name.as_str()) {
        errors.push(ConfigError::DeviceNameNotFound {
            device_name: config.device_name.clone(),
        });
    }

    for monitor in &config.monitors {
        for device_id in monitor.inputs.keys() {
            if !device_ids.contains(device_id.as_str()) {
                errors.push(ConfigError::UnknownDeviceInMonitorInput {
                    monitor_id: monitor.id.clone(),
                    device_id: device_id.clone(),
                });
            }
        }
    }

    let monitors_by_id: HashMap<&str, &Monitor> =
        config.monitors.iter().map(|m| (m.id.as_str(), m)).collect();

    for (preset_name, preset) in &config.presets {
        for (monitor_id, device_id) in &preset.layout {
            let Some(monitor) = monitors_by_id.get(monitor_id.as_str()) else {
                errors.push(ConfigError::UnknownMonitorInPresetLayout {
                    preset_name: preset_name.clone(),
                    monitor_id: monitor_id.clone(),
                });
                continue;
            };

            if !device_ids.contains(device_id.as_str()) {
                errors.push(ConfigError::UnknownDeviceInPresetLayout {
                    preset_name: preset_name.clone(),
                    monitor_id: monitor_id.clone(),
                    device_id: device_id.clone(),
                });
            } else if !monitor.inputs.contains_key(device_id.as_str()) {
                errors.push(ConfigError::DeviceNotInputForMonitor {
                    preset_name: preset_name.clone(),
                    monitor_id: monitor_id.clone(),
                    device_id: device_id.clone(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConfigErrors(errors))
    }
}
