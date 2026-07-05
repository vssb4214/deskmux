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
