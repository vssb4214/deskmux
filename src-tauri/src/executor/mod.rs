mod backend;
pub mod discovery;
mod model;
mod native;
mod resolve;
mod runner;
#[cfg(target_os = "windows")]
mod windows_ddc;

/// VCP feature code for input-source select — the only capability the config schema exposes
/// through `nativeDdc` today (see `InputNativeDdc` in config/model.rs), and the code the
/// discovery module reads.
pub(crate) const VCP_INPUT_SOURCE: u8 = 0x60;

use std::io;

use backend::{Backend, BackendAction, ShellBackend};
#[cfg(target_os = "windows")]
use native::{NativeDdcBackend, NativeDdcController};
use resolve::{preset_layout_entries, resolve_layout_entries, ResolvedEntry};
use runner::{CommandOutput, CommandRunner, ShellCommandRunner};
#[cfg(target_os = "windows")]
use windows_ddc::WindowsDdcController;

pub use model::{ExecutorError, MonitorOutcome, MonitorResult, ResolutionError};
pub use resolve::LayoutEntry;

use crate::config::Config;

/// Lists detected native-DDC displays by their `displayId`, for discoverability — copy the
/// value you want into `monitors[].nativeDdc.displayId` in your config. Empty on non-Windows
/// builds or if enumeration fails; this is best-effort diagnostics, not a hard dependency.
pub fn list_native_display_ids() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        WindowsDdcController
            .list_displays()
            .map(|displays| displays.into_iter().map(|d| d.display_id).collect())
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

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
    let shell = ShellCommandRunner;
    apply_preset_with_backend(
        config,
        preset_name,
        dry_run,
        NATIVE_DDC_AVAILABLE,
        &DefaultBackend::new(&shell),
    )
}

/// Resolves and runs only the supplied layout entries. Resolution failures are returned
/// per entry; the caller chooses which entries to include (filter before calling).
pub fn apply_layout_entries(
    config: &Config,
    entries: &[LayoutEntry],
    dry_run: bool,
) -> Vec<MonitorResult> {
    let shell = ShellCommandRunner;
    apply_layout_entries_with_backend(
        config,
        entries,
        dry_run,
        NATIVE_DDC_AVAILABLE,
        &DefaultBackend::new(&shell),
    )
}

/// Whether this build can execute `BackendAction::NativeDdc` at all.
const NATIVE_DDC_AVAILABLE: bool = cfg!(target_os = "windows");

/// Dispatches each `BackendAction` to the backend that can run it: shell always, native DDC
/// only on Windows (where `windows_ddc` is compiled in at all). Both backends share the same
/// injected `CommandRunner` seam for shell commands; native DDC has its own seam
/// (`native::NativeDdcController`).
struct DefaultBackend<'a> {
    shell: ShellBackend<'a>,
    #[cfg(target_os = "windows")]
    native: NativeDdcBackend<WindowsDdcController>,
}

impl<'a> DefaultBackend<'a> {
    fn new(runner: &'a dyn CommandRunner) -> Self {
        Self {
            shell: ShellBackend::new(runner),
            #[cfg(target_os = "windows")]
            native: NativeDdcBackend::new(WindowsDdcController),
        }
    }
}

impl Backend for DefaultBackend<'_> {
    fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput> {
        match action {
            BackendAction::Shell(_) => self.shell.execute(action),
            BackendAction::NativeDdc { .. } => {
                #[cfg(target_os = "windows")]
                {
                    self.native.execute(action)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "native DDC control is not available on this platform",
                    ))
                }
            }
        }
    }
}

fn apply_preset_with_backend(
    config: &Config,
    preset_name: &str,
    dry_run: bool,
    native_available: bool,
    backend: &dyn Backend,
) -> Result<Vec<MonitorResult>, ExecutorError> {
    let entries = preset_layout_entries(config, preset_name)?;
    Ok(apply_layout_entries_with_backend(
        config,
        &entries,
        dry_run,
        native_available,
        backend,
    ))
}

fn apply_layout_entries_with_backend(
    config: &Config,
    entries: &[LayoutEntry],
    dry_run: bool,
    native_available: bool,
    backend: &dyn Backend,
) -> Vec<MonitorResult> {
    resolve_layout_entries(config, entries, native_available)
        .into_iter()
        .map(|entry| run_entry(entry, dry_run, backend))
        .collect()
}

fn run_entry(entry: ResolvedEntry, dry_run: bool, backend: &dyn Backend) -> MonitorResult {
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
                is_native_ddc: false,
                outcome: MonitorOutcome::ResolutionFailed { error },
            };
        }
        ResolvedEntry::Ready(cmd) => cmd,
    };

    let display_command = cmd.action.display_command();
    let is_native_ddc = cmd.action.is_native_ddc();

    if dry_run {
        return MonitorResult {
            monitor_id: cmd.monitor_id,
            device_id: cmd.device_id,
            command: Some(display_command),
            executed: false,
            is_native_ddc,
            outcome: MonitorOutcome::DryRun,
        };
    }

    match backend.execute(&cmd.action) {
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
                command: Some(display_command),
                executed: true,
                is_native_ddc,
                outcome,
            }
        }
        Err(e) => MonitorResult {
            monitor_id: cmd.monitor_id,
            device_id: cmd.device_id,
            command: Some(display_command),
            executed: true,
            is_native_ddc,
            outcome: MonitorOutcome::SpawnFailed {
                message: e.to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::backend::BackendAction;
    use crate::executor::runner::CommandOutput;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io;

    struct MockBackend {
        responses: RefCell<VecDeque<io::Result<CommandOutput>>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockBackend {
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

    impl Backend for MockBackend {
        fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput> {
            self.calls.borrow_mut().push(action.display_command());
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

    fn native_fixture_config() -> Config {
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
                    "nativeDdc": { "displayId": "DEL4176:0" },
                    "inputs": {
                        "device-a": {
                            "type": "displayport",
                            "nativeDdc": { "inputSourceValue": 15 }
                        }
                    }
                }
            ],
            "presets": {
                "native_preset": {
                    "label": "Native",
                    "layout": { "monitor1": "device-a" }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    /// End-to-end through the real public API (no mock): a NativeDdc action against a
    /// displayId that doesn't exist on this machine must fail cleanly, never panic, on any
    /// platform — Windows resolves it as "display not found"; elsewhere DefaultBackend itself
    /// reports "not available on this platform". Either way, no crash and no silent success.
    #[test]
    fn native_ddc_preset_through_public_api_never_panics() {
        let config = native_fixture_config();

        let results = apply_preset(&config, "native_preset", false).expect("preset should resolve");

        assert_eq!(results.len(), 1);
        assert!(!matches!(
            results[0].outcome,
            MonitorOutcome::Success { .. }
        ));
    }

    #[test]
    fn apply_layout_entries_only_runs_filtered_monitors() {
        let config = fixture_config();
        let mock = MockBackend::new(vec![success("ok")]);
        let entries = vec![("monitor1".to_string(), "device-a".to_string())];

        let results = apply_layout_entries_with_backend(&config, &entries, false, false, &mock);

        assert_eq!(mock.call_count(), 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].monitor_id, "monitor1");
    }

    #[test]
    fn dry_run_never_spawns_a_process() {
        let config = fixture_config();
        let mock = MockBackend::new(vec![]);

        let results = apply_preset_with_backend(&config, "valid_preset", true, false, &mock)
            .expect("should resolve");

        assert_eq!(mock.call_count(), 0);
        assert!(results
            .iter()
            .all(|r| !r.executed && r.outcome == MonitorOutcome::DryRun));
    }

    #[test]
    fn valid_preset_resolves_to_expected_commands() {
        let config = fixture_config();
        let mock = MockBackend::new(vec![]);

        let results = apply_preset_with_backend(&config, "valid_preset", true, false, &mock)
            .expect("should resolve");

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
        let mock = MockBackend::new(vec![]);

        let result = apply_preset_with_backend(&config, "does-not-exist", false, false, &mock);

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
        let mock = MockBackend::new(vec![]);

        let results =
            apply_preset_with_backend(&config, "unknown_monitor_preset", false, false, &mock)
                .expect("resolution failures are reported per-entry, not a top-level error");

        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            results,
            vec![MonitorResult {
                monitor_id: "ghost-monitor".to_string(),
                device_id: "device-a".to_string(),
                command: None,
                executed: false,
                is_native_ddc: false,
                outcome: MonitorOutcome::ResolutionFailed {
                    error: ResolutionError::UnknownMonitor {
                        monitor_id: "ghost-monitor".to_string(),
                    },
                },
            }]
        );
    }

    #[test]
    fn mixed_result_preset_reports_each_entry_independently() {
        let config = fixture_config();
        let mock = MockBackend::new(vec![success("ok"), failure(1, "nope")]);

        let results = apply_preset_with_backend(&config, "mixed_preset", false, false, &mock)
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
                    is_native_ddc: false,
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
                    is_native_ddc: false,
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
                    is_native_ddc: false,
                    outcome: MonitorOutcome::ResolutionFailed {
                        error: ResolutionError::UnknownMonitor {
                            monitor_id: "ghost-monitor".to_string(),
                        },
                    },
                },
            ]
        );
    }
}
