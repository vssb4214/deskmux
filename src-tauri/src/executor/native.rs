use std::io;

#[cfg(target_os = "windows")]
use super::backend::{Backend, BackendAction};
#[cfg(target_os = "windows")]
use super::runner::CommandOutput;

/// A physical display DeskMux can address for native DDC/CI control.
pub(super) struct NativeDisplay {
    pub display_id: String,
}

/// Native DDC features DeskMux knows how to address. This is deliberately bounded: callers
/// choose a named feature, never a raw VCP code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDdcFeature {
    InputSource,
    Brightness,
    Contrast,
    Volume,
}

impl NativeDdcFeature {
    pub fn label(self) -> &'static str {
        match self {
            NativeDdcFeature::InputSource => "input source",
            NativeDdcFeature::Brightness => "brightness",
            NativeDdcFeature::Contrast => "contrast",
            NativeDdcFeature::Volume => "volume",
        }
    }

    pub fn api_name(self) -> &'static str {
        match self {
            NativeDdcFeature::InputSource => "inputSource",
            NativeDdcFeature::Brightness => "brightness",
            NativeDdcFeature::Contrast => "contrast",
            NativeDdcFeature::Volume => "volume",
        }
    }

    pub fn continuous_controls() -> [NativeDdcFeature; 3] {
        [
            NativeDdcFeature::Brightness,
            NativeDdcFeature::Contrast,
            NativeDdcFeature::Volume,
        ]
    }

    pub fn is_continuous_control(self) -> bool {
        matches!(
            self,
            NativeDdcFeature::Brightness | NativeDdcFeature::Contrast | NativeDdcFeature::Volume
        )
    }
}

/// One VCP feature read: the current value and the maximum the monitor reports. Note the
/// maximum is a single number, not a list of supported values — real hardware reports e.g.
/// `current=4626, maximum=4626` for input-source. Discovering which value maps to which
/// physical input is done by switching inputs and re-reading `current`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VcpReading {
    pub current: u32,
    pub maximum: u32,
}

/// The low-level operations needed to control a monitor's input over DDC/CI, behind a trait so
/// tests can substitute a fake implementation instead of calling real Windows APIs.
pub(super) trait NativeDdcController {
    /// Enumerates currently connected displays this controller can address.
    fn list_displays(&self) -> io::Result<Vec<NativeDisplay>>;
    /// Writes `value` to VCP feature `vcp_code` on the display identified by `display_id`.
    /// Only called by `NativeDdcBackend` (Windows-gated) — genuinely unused on other platforms
    /// today, since `executor::discovery` is read-only. Not `#[cfg(target_os = "windows")]` on
    /// the trait itself so the interface stays uniform for every implementor (including
    /// discovery's cross-platform test doubles); the allow documents *why* it's silenced rather
    /// than hiding an actual bug.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn set_vcp_feature(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
        value: u16,
    ) -> io::Result<()>;
    /// Reads VCP feature `vcp_code` from the display identified by `display_id`. A single
    /// attempt — retry policy lives in `executor::discovery`, not here.
    fn get_vcp_feature(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
    ) -> io::Result<VcpReading>;
}

/// Runs `BackendAction::NativeDdc` via an injected [`NativeDdcController`] — real Windows calls
/// in production, a mock in tests. Windows-gated because its only production consumer
/// (`DefaultBackend`'s native arm) is; the trait and types above stay cross-platform for
/// `executor::discovery` and its tests.
#[cfg(target_os = "windows")]
pub(super) struct NativeDdcBackend<C: NativeDdcController> {
    controller: C,
}

#[cfg(target_os = "windows")]
impl<C: NativeDdcController> NativeDdcBackend<C> {
    pub(super) fn new(controller: C) -> Self {
        Self { controller }
    }
}

#[cfg(target_os = "windows")]
impl<C: NativeDdcController> Backend for NativeDdcBackend<C> {
    fn execute(&self, action: &BackendAction) -> io::Result<CommandOutput> {
        let BackendAction::NativeDdc {
            display_id,
            feature,
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
            .set_vcp_feature(display_id, *feature, *value)
        {
            Ok(()) => Ok(CommandOutput {
                success: true,
                exit_code: None,
                stdout: format!(
                    "set native DDC {} = {value} on display '{display_id}'",
                    feature.label()
                ),
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockController {
        displays: Vec<NativeDisplay>,
        set_vcp_result: RefCell<Option<io::Result<()>>>,
        set_vcp_calls: RefCell<Vec<(String, NativeDdcFeature, u16)>>,
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

        fn set_vcp_feature(
            &self,
            display_id: &str,
            feature: NativeDdcFeature,
            value: u16,
        ) -> io::Result<()> {
            self.set_vcp_calls
                .borrow_mut()
                .push((display_id.to_string(), feature, value));
            self.set_vcp_result
                .borrow_mut()
                .take()
                .expect("set_vcp_feature called more than once in this test")
        }

        fn get_vcp_feature(
            &self,
            _display_id: &str,
            _feature: NativeDdcFeature,
        ) -> io::Result<VcpReading> {
            unreachable!("NativeDdcBackend never reads; discovery has its own mock")
        }
    }

    fn native_action(display_id: &str, value: u16) -> BackendAction {
        BackendAction::NativeDdc {
            display_id: display_id.to_string(),
            feature: NativeDdcFeature::InputSource,
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
            [("DEL4176:0".to_string(), NativeDdcFeature::InputSource, 15)]
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
