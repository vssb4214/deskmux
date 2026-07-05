use std::io;

use super::backend::{Backend, BackendAction};
use super::runner::CommandOutput;

/// A physical display DeskMux can address for native DDC/CI control.
pub(super) struct NativeDisplay {
    pub display_id: String,
}

/// The low-level operations needed to control a monitor's input over DDC/CI, behind a trait so
/// tests can substitute a fake implementation instead of calling real Windows APIs.
pub(super) trait NativeDdcController {
    /// Enumerates currently connected displays this controller can address.
    fn list_displays(&self) -> io::Result<Vec<NativeDisplay>>;
    /// Writes `value` to VCP feature `vcp_code` on the display identified by `display_id`.
    fn set_vcp_feature(&self, display_id: &str, vcp_code: u8, value: u16) -> io::Result<()>;
}

/// Runs `BackendAction::NativeDdc` via an injected [`NativeDdcController`] — real Windows calls
/// in production, a mock in tests.
pub(super) struct NativeDdcBackend<C: NativeDdcController> {
    controller: C,
}

impl<C: NativeDdcController> NativeDdcBackend<C> {
    pub(super) fn new(controller: C) -> Self {
        Self { controller }
    }
}

impl<C: NativeDdcController> Backend for NativeDdcBackend<C> {
    fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput> {
        let BackendAction::NativeDdc {
            display_id,
            vcp_code,
            value,
        } = action
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "NativeDdcBackend received a non-native action",
            ));
        };

        // "Couldn't find the display" is a plumbing failure (nothing to write to) - Err, maps
        // to SpawnFailed, same as a shell command that never started.
        let displays = self.controller.list_displays()?;
        if !displays.iter().any(|d| d.display_id == *display_id) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no display found matching displayId '{display_id}'"),
            ));
        }

        // Found the display; whether the VCP write itself succeeds is reported as a normal
        // outcome (Ok with success: false on failure), same as a shell command exiting non-zero
        // - it ran, it just didn't succeed.
        match self
            .controller
            .set_vcp_feature(display_id, *vcp_code, *value)
        {
            Ok(()) => Ok(CommandOutput {
                success: true,
                exit_code: None,
                stdout: format!("set VCP 0x{vcp_code:02x} = {value} on display '{display_id}'"),
                stderr: String::new(),
            }),
            Err(e) => Ok(CommandOutput {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockController {
        displays: Vec<NativeDisplay>,
        set_vcp_result: RefCell<Option<io::Result<()>>>,
        set_vcp_calls: RefCell<Vec<(String, u8, u16)>>,
    }

    impl MockController {
        fn new(displays: Vec<&str>, set_vcp_result: io::Result<()>) -> Self {
            Self {
                displays: displays
                    .into_iter()
                    .map(|id| NativeDisplay {
                        display_id: id.to_string(),
                    })
                    .collect(),
                set_vcp_result: RefCell::new(Some(set_vcp_result)),
                set_vcp_calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl NativeDdcController for MockController {
        fn list_displays(&self) -> io::Result<Vec<NativeDisplay>> {
            Ok(self
                .displays
                .iter()
                .map(|d| NativeDisplay {
                    display_id: d.display_id.clone(),
                })
                .collect())
        }

        fn set_vcp_feature(&self, display_id: &str, vcp_code: u8, value: u16) -> io::Result<()> {
            self.set_vcp_calls
                .borrow_mut()
                .push((display_id.to_string(), vcp_code, value));
            self.set_vcp_result
                .borrow_mut()
                .take()
                .expect("set_vcp_feature called more than once in this test")
        }
    }

    fn native_action(display_id: &str, value: u16) -> BackendAction {
        BackendAction::NativeDdc {
            display_id: display_id.to_string(),
            vcp_code: 0x60,
            value,
        }
    }

    #[test]
    fn writes_vcp_feature_on_matching_display() {
        let controller = MockController::new(vec!["DEL4176:0"], Ok(()));
        let backend = NativeDdcBackend::new(controller);

        let output = backend
            .execute(&native_action("DEL4176:0", 15))
            .expect("should execute");

        assert!(output.success);
        assert_eq!(
            backend.controller.set_vcp_calls.borrow().as_slice(),
            [("DEL4176:0".to_string(), 0x60, 15)]
        );
    }

    #[test]
    fn missing_display_is_an_io_error_not_a_vcp_attempt() {
        let controller = MockController::new(vec!["OTHER:0"], Ok(()));
        let backend = NativeDdcBackend::new(controller);

        let result = backend.execute(&native_action("DEL4176:0", 15));

        assert!(result.is_err());
        assert!(backend.controller.set_vcp_calls.borrow().is_empty());
    }

    #[test]
    fn vcp_write_failure_is_a_failed_outcome_not_an_error() {
        let controller = MockController::new(
            vec!["DEL4176:0"],
            Err(io::Error::other("monitor rejected write")),
        );
        let backend = NativeDdcBackend::new(controller);

        let output = backend
            .execute(&native_action("DEL4176:0", 15))
            .expect("display was found; this should be Ok(success: false)");

        assert!(!output.success);
        assert!(output.stderr.contains("monitor rejected write"));
    }

    #[test]
    fn non_native_action_is_rejected() {
        let controller = MockController::new(vec!["DEL4176:0"], Ok(()));
        let backend = NativeDdcBackend::new(controller);

        let result = backend.execute(&BackendAction::Shell("echo hi".to_string()));

        assert!(result.is_err());
    }
}
