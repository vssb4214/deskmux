use std::io;

use super::runner::{CommandOutput, CommandRunner};

/// What a resolved layout entry should do to switch an input.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum BackendAction {
    Shell(String),
    /// Write `value` to VCP feature `vcp_code` on the display identified by `display_id`.
    /// Generically VCP-shaped at this layer since the underlying call is the same for any VCP
    /// code — but the *config* schema only ever produces `vcp_code: 0x60` (input-source) today;
    /// see `InputNativeDdc` in config/model.rs for that boundary.
    NativeDdc {
        display_id: String,
        vcp_code: u8,
        value: u16,
    },
}

impl BackendAction {
    /// Human-readable form for `MonitorResult.command`.
    pub(super) fn display_command(&self) -> String {
        match self {
            BackendAction::Shell(command) => command.clone(),
            BackendAction::NativeDdc {
                display_id,
                vcp_code,
                value,
            } => format!("native DDC: display '{display_id}' VCP 0x{vcp_code:02x} = {value}"),
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
            // Resolution should never hand a NativeDdc action to the shell backend — routing
            // to the right backend per action is DefaultBackend's job (see mod.rs). Fail
            // clearly rather than silently misbehaving if that invariant is ever violated.
            BackendAction::NativeDdc { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ShellBackend received a NativeDdc action",
            )),
        }
    }
}
