mod model;
mod resolve;
mod runner;

use resolve::{resolve_preset, ResolvedEntry};
use runner::{CommandRunner, ShellCommandRunner};

pub use model::{ExecutorError, MonitorOutcome, MonitorResult, ResolutionError};

use crate::config::Config;

/// Resolves `preset_name`'s layout against `config` and runs each monitor's command
/// sequentially. When `dry_run` is true, commands are resolved and returned but never spawned.
/// A layout entry that can't be resolved (unknown monitor, or a device with no command
/// configured on that monitor) doesn't abort the rest of the preset — it's reported as its own
/// failed `MonitorResult` alongside the others.
pub fn apply_preset(
    config: &Config,
    preset_name: &str,
    dry_run: bool,
) -> Result<Vec<MonitorResult>, ExecutorError> {
    apply_preset_with_runner(config, preset_name, dry_run, &ShellCommandRunner)
}

fn apply_preset_with_runner(
    config: &Config,
    preset_name: &str,
    dry_run: bool,
    runner: &dyn CommandRunner,
) -> Result<Vec<MonitorResult>, ExecutorError> {
    let entries = resolve_preset(config, preset_name)?;

    let results = entries
        .into_iter()
        .map(|entry| run_entry(entry, dry_run, runner))
        .collect();

    Ok(results)
}

fn run_entry(entry: ResolvedEntry, dry_run: bool, runner: &dyn CommandRunner) -> MonitorResult {
    let cmd = match entry {
        ResolvedEntry::Failed {
            monitor_id,
            device_id,
            error,
        } => {
            return MonitorResult {
                monitor_id,
                device_id,
                command: None,
                executed: false,
                outcome: MonitorOutcome::ResolutionFailed(error),
            };
        }
        ResolvedEntry::Ready(cmd) => cmd,
    };

    if dry_run {
        return MonitorResult {
            monitor_id: cmd.monitor_id,
            device_id: cmd.device_id,
            command: Some(cmd.command),
            executed: false,
            outcome: MonitorOutcome::DryRun,
        };
    }

    match runner.run(&cmd.command) {
        Ok(output) => {
            let outcome = if output.success {
                MonitorOutcome::Success {
                    stdout: output.stdout,
                    stderr: output.stderr,
                }
            } else {
                MonitorOutcome::Failed {
                    stdout: output.stdout,
                    stderr: output.stderr,
                    exit_code: output.exit_code,
                }
            };
            MonitorResult {
                monitor_id: cmd.monitor_id,
                device_id: cmd.device_id,
                command: Some(cmd.command),
                executed: true,
                outcome,
            }
        }
        Err(e) => MonitorResult {
            monitor_id: cmd.monitor_id,
            device_id: cmd.device_id,
            command: Some(cmd.command),
            executed: true,
            outcome: MonitorOutcome::SpawnFailed {
                message: e.to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::runner::CommandOutput;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io;

    struct MockCommandRunner {
        responses: RefCell<VecDeque<io::Result<CommandOutput>>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockCommandRunner {
        fn new(responses: Vec<io::Result<CommandOutput>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.borrow().len()
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn run(&self, command: &str) -> io::Result<CommandOutput> {
            self.calls.borrow_mut().push(command.to_string());
            self.responses
                .borrow_mut()
                .pop_front()
                .expect("mock ran out of queued responses")
        }
    }

    fn success(stdout: &str) -> io::Result<CommandOutput> {
        Ok(CommandOutput {
            success: true,
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }

    fn failure(exit_code: i32, stderr: &str) -> io::Result<CommandOutput> {
        Ok(CommandOutput {
            success: false,
            exit_code: Some(exit_code),
            stdout: String::new(),
            stderr: stderr.to_string(),
        })
    }

    fn fixture_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [
                { "id": "device-a", "label": "Device A" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "Monitor 1",
                    "order": 0,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-monitor1-a" }
                    }
                },
                {
                    "id": "monitor2",
                    "label": "Monitor 2",
                    "order": 1,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-monitor2-a" }
                    }
                }
            ],
            "presets": {
                "valid_preset": {
                    "label": "Valid",
                    "layout": { "monitor1": "device-a", "monitor2": "device-a" }
                },
                "unknown_monitor_preset": {
                    "label": "Unknown monitor",
                    "layout": { "ghost-monitor": "device-a" }
                },
                "mixed_preset": {
                    "label": "Mixed",
                    "layout": {
                        "monitor1": "device-a",
                        "monitor2": "device-a",
                        "ghost-monitor": "device-a"
                    }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    #[test]
    fn dry_run_never_spawns_a_process() {
        let config = fixture_config();
        let mock = MockCommandRunner::new(vec![]);

        let results =
            apply_preset_with_runner(&config, "valid_preset", true, &mock).expect("should resolve");

        assert_eq!(mock.call_count(), 0);
        assert!(results
            .iter()
            .all(|r| !r.executed && r.outcome == MonitorOutcome::DryRun));
    }

    #[test]
    fn valid_preset_resolves_to_expected_commands() {
        let config = fixture_config();
        let mock = MockCommandRunner::new(vec![]);

        let results =
            apply_preset_with_runner(&config, "valid_preset", true, &mock).expect("should resolve");

        assert_eq!(
            results
                .iter()
                .map(|r| (r.monitor_id.as_str(), r.command.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("monitor1", Some("cmd-monitor1-a")),
                ("monitor2", Some("cmd-monitor2-a")),
            ]
        );
    }

    #[test]
    fn unknown_preset_name_bubbles_up_as_executor_error() {
        let config = fixture_config();
        let mock = MockCommandRunner::new(vec![]);

        let result = apply_preset_with_runner(&config, "does-not-exist", false, &mock);

        assert_eq!(
            result.unwrap_err(),
            ExecutorError::PresetNotFound {
                preset_name: "does-not-exist".to_string(),
            }
        );
    }

    #[test]
    fn resolution_failure_is_a_structured_result_not_a_panic() {
        let config = fixture_config();
        let mock = MockCommandRunner::new(vec![]);

        let results = apply_preset_with_runner(&config, "unknown_monitor_preset", false, &mock)
            .expect("resolution failures are reported per-entry, not a top-level error");

        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            results,
            vec![MonitorResult {
                monitor_id: "ghost-monitor".to_string(),
                device_id: "device-a".to_string(),
                command: None,
                executed: false,
                outcome: MonitorOutcome::ResolutionFailed(ResolutionError::UnknownMonitor {
                    monitor_id: "ghost-monitor".to_string(),
                }),
            }]
        );
    }

    #[test]
    fn mixed_result_preset_reports_each_entry_independently() {
        let config = fixture_config();
        let mock = MockCommandRunner::new(vec![success("ok"), failure(1, "nope")]);

        let results = apply_preset_with_runner(&config, "mixed_preset", false, &mock)
            .expect("should resolve");

        assert_eq!(mock.call_count(), 2);
        assert_eq!(
            results,
            vec![
                MonitorResult {
                    monitor_id: "monitor1".to_string(),
                    device_id: "device-a".to_string(),
                    command: Some("cmd-monitor1-a".to_string()),
                    executed: true,
                    outcome: MonitorOutcome::Success {
                        stdout: "ok".to_string(),
                        stderr: String::new(),
                    },
                },
                MonitorResult {
                    monitor_id: "monitor2".to_string(),
                    device_id: "device-a".to_string(),
                    command: Some("cmd-monitor2-a".to_string()),
                    executed: true,
                    outcome: MonitorOutcome::Failed {
                        stdout: String::new(),
                        stderr: "nope".to_string(),
                        exit_code: Some(1),
                    },
                },
                MonitorResult {
                    monitor_id: "ghost-monitor".to_string(),
                    device_id: "device-a".to_string(),
                    command: None,
                    executed: false,
                    outcome: MonitorOutcome::ResolutionFailed(ResolutionError::UnknownMonitor {
                        monitor_id: "ghost-monitor".to_string(),
                    }),
                },
            ]
        );
    }
}
