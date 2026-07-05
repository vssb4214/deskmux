use std::collections::{HashMap, HashSet};

use super::model::{PlanningError, PlanningErrorKind};
use crate::config::Config;
use crate::executor::{ExecutorError, LayoutEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyScope {
    /// Full coordinator: missing monitors in this config are planning errors.
    Coordinated,
    /// Peer/local-only: skip monitors missing from this config.
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPlan {
    pub local_entries: Vec<LayoutEntry>,
    /// Distinct remote `controlledBy` device ids requiring peer calls, sorted.
    pub remote_peers: Vec<String>,
    pub planning_errors: Vec<PlanningError>,
}

pub fn plan_apply(
    config: &Config,
    preset_name: &str,
    scope: ApplyScope,
) -> Result<ApplyPlan, ExecutorError> {
    let preset = config
        .presets
        .get(preset_name)
        .ok_or_else(|| ExecutorError::PresetNotFound {
            preset_name: preset_name.to_string(),
        })?;

    let monitors_by_id: HashMap<&str, _> =
        config.monitors.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut local_entries = Vec::new();
    let mut remote_peers = HashSet::new();
    let mut planning_errors = Vec::new();

    for (monitor_id, device_id) in &preset.layout {
        let Some(monitor) = monitors_by_id.get(monitor_id.as_str()) else {
            if scope == ApplyScope::Coordinated {
                planning_errors.push(PlanningError {
                    monitor_id: monitor_id.clone(),
                    kind: PlanningErrorKind::UnknownMonitor,
                });
            }
            continue;
        };

        let owner = monitor.controlled_by(&config.device_name);
        if owner == config.device_name.as_str() {
            local_entries.push((monitor_id.clone(), device_id.clone()));
        } else if scope == ApplyScope::Coordinated {
            remote_peers.insert(owner.to_string());
        }
    }

    let mut remote_peers: Vec<String> = remote_peers.into_iter().collect();
    remote_peers.sort();

    Ok(ApplyPlan {
        local_entries,
        remote_peers,
        planning_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn coordinator_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [{ "name": "device-b", "host": "192.168.1.2", "port": 3737 }],
            "devices": [
                { "id": "device-a", "label": "A" },
                { "id": "device-b", "label": "B" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "M1",
                    "order": 0,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-m1-a" }
                    }
                },
                {
                    "id": "monitor2",
                    "label": "M2",
                    "order": 1,
                    "controlledBy": "device-b"
                }
            ],
            "presets": {
                "split": {
                    "label": "Split",
                    "layout": { "monitor1": "device-a", "monitor2": "device-b" }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    #[test]
    fn coordinated_plan_partitions_by_controlled_by() {
        let plan = plan_apply(&coordinator_config(), "split", ApplyScope::Coordinated)
            .expect("plan should succeed");

        assert_eq!(
            plan.local_entries,
            vec![("monitor1".to_string(), "device-a".to_string())]
        );
        assert_eq!(plan.remote_peers, vec!["device-b".to_string()]);
        assert!(plan.planning_errors.is_empty());
    }

    #[test]
    fn coordinated_plan_errors_on_missing_monitor_stub() {
        let mut config = coordinator_config();
        config
            .presets
            .get_mut("split")
            .unwrap()
            .layout
            .insert("ghost".to_string(), "device-a".to_string());

        let plan = plan_apply(&config, "split", ApplyScope::Coordinated).expect("plan succeeds");

        assert_eq!(
            plan.planning_errors,
            vec![PlanningError {
                monitor_id: "ghost".to_string(),
                kind: PlanningErrorKind::UnknownMonitor,
            }]
        );
    }

    #[test]
    fn local_only_plan_skips_missing_and_remote_monitors() {
        let plan = plan_apply(&coordinator_config(), "split", ApplyScope::LocalOnly)
            .expect("plan should succeed");

        assert_eq!(
            plan.local_entries,
            vec![("monitor1".to_string(), "device-a".to_string())]
        );
        assert!(plan.remote_peers.is_empty());
        assert!(plan.planning_errors.is_empty());
    }
}
