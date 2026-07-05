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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Device, Input, Peer, Preset};

    fn valid_config() -> Config {
        let mut inputs = HashMap::new();
        inputs.insert(
            "device-a".to_string(),
            Input {
                kind: "hdmi".to_string(),
                command: "cmd-a".to_string(),
            },
        );

        let mut layout = HashMap::new();
        layout.insert("monitor1".to_string(), "device-a".to_string());

        let mut presets = HashMap::new();
        presets.insert(
            "all_a".to_string(),
            Preset {
                label: "All A".to_string(),
                layout,
            },
        );

        Config {
            device_name: "device-a".to_string(),
            api_port: 3737,
            api_lan_access: false,
            peers: vec![Peer {
                name: "device-b".to_string(),
                host: "192.168.1.2".to_string(),
                port: 3737,
            }],
            devices: vec![
                Device {
                    id: "device-a".to_string(),
                    label: "Device A".to_string(),
                },
                Device {
                    id: "device-b".to_string(),
                    label: "Device B".to_string(),
                },
            ],
            monitors: vec![Monitor {
                id: "monitor1".to_string(),
                label: "Monitor 1".to_string(),
                order: 0,
                inputs,
            }],
            presets,
        }
    }

    #[test]
    fn valid_config_passes() {
        assert!(validate(&valid_config()).is_ok());
    }

    #[test]
    fn rejects_unknown_device_name() {
        let mut config = valid_config();
        config.device_name = "ghost".to_string();

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::DeviceNameNotFound {
                device_name: "ghost".to_string()
            }]
        );
    }

    #[test]
    fn rejects_duplicate_device_id() {
        let mut config = valid_config();
        config.devices.push(Device {
            id: "device-a".to_string(),
            label: "Duplicate".to_string(),
        });

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::DuplicateDeviceId {
                device_id: "device-a".to_string()
            }]
        );
    }

    #[test]
    fn rejects_duplicate_monitor_id() {
        let mut config = valid_config();
        let duplicate = config.monitors[0].clone();
        config.monitors.push(duplicate);

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::DuplicateMonitorId {
                monitor_id: "monitor1".to_string()
            }]
        );
    }

    #[test]
    fn rejects_unknown_device_in_monitor_input() {
        let mut config = valid_config();
        config.monitors[0].inputs.insert(
            "ghost-device".to_string(),
            Input {
                kind: "hdmi".to_string(),
                command: "cmd".to_string(),
            },
        );

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::UnknownDeviceInMonitorInput {
                monitor_id: "monitor1".to_string(),
                device_id: "ghost-device".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_unknown_monitor_in_preset_layout() {
        let mut config = valid_config();
        config
            .presets
            .get_mut("all_a")
            .unwrap()
            .layout
            .insert("ghost-monitor".to_string(), "device-a".to_string());

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::UnknownMonitorInPresetLayout {
                preset_name: "all_a".to_string(),
                monitor_id: "ghost-monitor".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_unknown_device_in_preset_layout() {
        let mut config = valid_config();
        config
            .presets
            .get_mut("all_a")
            .unwrap()
            .layout
            .insert("monitor1".to_string(), "ghost-device".to_string());

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::UnknownDeviceInPresetLayout {
                preset_name: "all_a".to_string(),
                monitor_id: "monitor1".to_string(),
                device_id: "ghost-device".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_device_not_declared_as_monitor_input() {
        let mut config = valid_config();
        // device-b exists in devices[] but monitor1 never declared an input for it.
        config
            .presets
            .get_mut("all_a")
            .unwrap()
            .layout
            .insert("monitor1".to_string(), "device-b".to_string());

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::DeviceNotInputForMonitor {
                preset_name: "all_a".to_string(),
                monitor_id: "monitor1".to_string(),
                device_id: "device-b".to_string(),
            }]
        );
    }

    #[test]
    fn collects_every_error_at_once() {
        let mut config = valid_config();
        config.device_name = "ghost".to_string();
        config.devices.push(Device {
            id: "device-a".to_string(),
            label: "Duplicate".to_string(),
        });

        let errors = validate(&config).unwrap_err();

        assert_eq!(errors.0.len(), 2);
    }
}
