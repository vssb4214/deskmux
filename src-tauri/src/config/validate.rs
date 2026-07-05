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
        if let Some(controlled_by) = &monitor.controlled_by {
            if !device_ids.contains(controlled_by.as_str()) {
                errors.push(ConfigError::UnknownControlledBy {
                    monitor_id: monitor.id.clone(),
                    controlled_by: controlled_by.clone(),
                });
            }
        }

        for device_id in monitor.inputs.keys() {
            if !device_ids.contains(device_id.as_str()) {
                errors.push(ConfigError::UnknownDeviceInMonitorInput {
                    monitor_id: monitor.id.clone(),
                    device_id: device_id.clone(),
                });
            }
        }

        if monitor.controlled_by(&config.device_name) == config.device_name.as_str()
            && monitor.inputs.is_empty()
        {
            errors.push(ConfigError::LocallyOwnedMonitorMissingInputs {
                monitor_id: monitor.id.clone(),
            });
        }
    }

    for peer in &config.peers {
        if !device_ids.contains(peer.name.as_str()) {
            errors.push(ConfigError::PeerNameNotFound {
                peer_name: peer.name.clone(),
            });
        } else if peer.name == config.device_name {
            errors.push(ConfigError::PeerNameIsLocalDevice {
                peer_name: peer.name.clone(),
            });
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
                continue;
            }

            if monitor.controlled_by(&config.device_name) == config.device_name.as_str()
                && !monitor.inputs.contains_key(device_id.as_str())
            {
                errors.push(ConfigError::DeviceNotInputForMonitor {
                    preset_name: preset_name.clone(),
                    monitor_id: monitor_id.clone(),
                    device_id: device_id.clone(),
                });
            }
        }
    }

    crate::hotkeys::validate_hotkeys(config, &mut errors);

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
                controlled_by: None,
                inputs,
            }],
            presets,
            hotkeys: HashMap::new(),
        }
    }

    #[test]
    fn valid_config_passes() {
        assert!(validate(&valid_config()).is_ok());
    }

    #[test]
    fn controlled_by_defaults_to_device_name_via_helper() {
        let config = valid_config();
        assert_eq!(
            config.monitors[0].controlled_by(&config.device_name),
            "device-a"
        );
    }

    #[test]
    fn remote_stub_monitor_without_inputs_passes() {
        let mut config = valid_config();
        config.monitors.push(Monitor {
            id: "monitor2".to_string(),
            label: "Monitor 2".to_string(),
            order: 1,
            controlled_by: Some("device-b".to_string()),
            inputs: HashMap::new(),
        });
        config
            .presets
            .get_mut("all_a")
            .unwrap()
            .layout
            .insert("monitor2".to_string(), "device-b".to_string());

        assert!(validate(&config).is_ok());
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
    fn rejects_unknown_controlled_by() {
        let mut config = valid_config();
        config.monitors[0].controlled_by = Some("ghost".to_string());

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::UnknownControlledBy {
                monitor_id: "monitor1".to_string(),
                controlled_by: "ghost".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_locally_owned_monitor_without_inputs() {
        let mut config = valid_config();
        config.monitors[0].inputs.clear();

        let errors = validate(&config).unwrap_err();

        assert!(errors
            .0
            .contains(&ConfigError::LocallyOwnedMonitorMissingInputs {
                monitor_id: "monitor1".to_string(),
            }));
    }

    #[test]
    fn rejects_peer_name_not_in_devices() {
        let mut config = valid_config();
        config.peers[0].name = "ghost".to_string();

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::PeerNameNotFound {
                peer_name: "ghost".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_peer_name_equal_to_device_name() {
        let mut config = valid_config();
        config.peers[0].name = "device-a".to_string();

        let errors = validate(&config).unwrap_err();

        assert_eq!(
            errors.0,
            vec![ConfigError::PeerNameIsLocalDevice {
                peer_name: "device-a".to_string(),
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
    fn rejects_device_not_declared_as_monitor_input_for_locally_owned_monitor() {
        let mut config = valid_config();
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
    fn allows_remote_owned_preset_without_local_input() {
        let mut config = valid_config();
        config.monitors.push(Monitor {
            id: "monitor2".to_string(),
            label: "Monitor 2".to_string(),
            order: 1,
            controlled_by: Some("device-b".to_string()),
            inputs: HashMap::new(),
        });
        config
            .presets
            .get_mut("all_a")
            .unwrap()
            .layout
            .insert("monitor2".to_string(), "device-b".to_string());

        assert!(validate(&config).is_ok());
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
