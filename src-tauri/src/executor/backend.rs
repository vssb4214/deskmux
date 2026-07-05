use std::io;

use super::runner::{CommandOutput, CommandRunner};

/// What a resolved layout entry should do to switch an input. Only one variant exists today,
/// because `Input` only ever describes a shell command — this is the seam a Phase 2 native DDC
/// action will slot into once an approach is chosen (see docs/ROADMAP.md).
#[derive(Debug, PartialEq)]
pub(super) enum BackendAction {
    Shell(String),
}

impl BackendAction {
    /// Human-readable form for `MonitorResult.command`. Today this is always the raw shell
    /// command string; a future native action would synthesize a description here instead.
    pub(super) fn display_command(&self) -> String {
        match self {
            BackendAction::Shell(command) => command.clone(),
        }
    }
}

/// Something that can carry out a resolved [`BackendAction`]. The shell-command executor is one
/// implementation; a native DDC backend is a future second implementation behind the same trait.
pub(super) trait Backend {
    fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput>;
}

/// Runs [`BackendAction::Shell`] via an injected [`CommandRunner`] (real shell-out in
/// production, a mock in tests). `CommandRunner` and `ShellCommandRunner` are unchanged from
/// before this trait existed — this just wraps them behind the backend-agnostic seam.
pub(super) struct ShellBackend<'a> {
    runner: &'a dyn CommandRunner,
}

impl<'a> ShellBackend<'a> {
    pub(super) fn new(runner: &'a dyn CommandRunner) -> Self {
        Self { runner }
    }
}

impl Backend for ShellBackend<'_> {
    fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput> {
        match action {
            BackendAction::Shell(command) => self.runner.run(command),
        }
    }
}
