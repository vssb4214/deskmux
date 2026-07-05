use std::io;
use std::process::Command;

/// Runner-agnostic view of a finished command, decoupled from `std::process::Output` so tests
/// can construct one directly instead of faking a platform-specific `ExitStatus`.
pub(super) struct CommandOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// The seam between preset execution and actually spawning a process, so tests can swap in a
/// mock instead of shelling out to a real monitor backend.
pub(super) trait CommandRunner {
    fn run(&self, command: &str) -> io::Result<CommandOutput>;
}

pub(super) struct ShellCommandRunner;

impl CommandRunner for ShellCommandRunner {
    fn run(&self, command: &str) -> io::Result<CommandOutput> {
        #[cfg(windows)]
        let output = Command::new("cmd").args(["/C", command]).output()?;

        #[cfg(not(windows))]
        let output = Command::new("sh").args(["-c", command]).output()?;

        Ok(CommandOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
