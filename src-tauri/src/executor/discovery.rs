//! In-app monitor discovery: enumerate connected displays, read the current VCP 0x60
//! input-source value, so a user can identify which value selects which physical input by
//! switching inputs on the monitor and re-reading — instead of hand-editing config from a
//! diagnostic session. Probe writes are intentionally narrow: one explicit VCP 0x60 write per
//! request for setup-time test switching (see docs/NATIVE_DDC_DISCOVERY.md).
//!
//! Note `GetVCPFeatureAndVCPFeatureReply` returns `(current, maximum)` — the maximum is a
//! single number, not a list of supported values, so identify-by-switching is the discovery
//! mechanism, not enumeration of options.

use std::fmt;
use std::io;

use super::native::NativeDdcController;
#[cfg(not(target_os = "windows"))]
use super::native::{NativeDisplay, VcpReading};
use super::VCP_INPUT_SOURCE;

/// A display found by native enumeration, identified by the same EDID-derived `displayId`
/// users put in `monitors[].nativeDdc.displayId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDisplay {
    pub display_id: String,
}

/// A successful VCP 0x60 read. `maximum` is the monitor-reported maximum value — a single
/// number (often equal to `current` on real hardware), not a list of valid inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSourceReading {
    pub current: u32,
    pub maximum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeInputResult {
    pub accepted: bool,
    pub current: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// Native DDC is not available on this platform (everything except Windows today).
    NativeUnavailable,
    /// Display enumeration itself failed.
    EnumerationFailed { detail: String },
    /// The display was not present in a fresh enumeration — not retried, because every lookup
    /// already re-enumerates; absent means absent.
    DisplayNotFound { display_id: String },
    /// The display was found but the VCP read failed, including after one refresh-and-retry.
    VcpReadFailed { detail: String },
    /// The display was found but a single VCP write attempt failed.
    VcpWriteFailed { detail: String },
}

impl DiscoveryError {
    /// Stable machine-readable code for API responses (see docs/NATIVE_DDC_DISCOVERY.md).
    pub fn code(&self) -> &'static str {
        match self {
            DiscoveryError::NativeUnavailable => "nativeUnavailable",
            DiscoveryError::EnumerationFailed { .. } => "enumerationFailed",
            DiscoveryError::DisplayNotFound { .. } => "displayNotFound",
            DiscoveryError::VcpReadFailed { .. } => "vcpReadFailed",
            DiscoveryError::VcpWriteFailed { .. } => "vcpWriteFailed",
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::NativeUnavailable => {
                write!(f, "native DDC discovery is not available on this platform")
            }
            DiscoveryError::EnumerationFailed { detail } => {
                write!(f, "display enumeration failed: {detail}")
            }
            DiscoveryError::DisplayNotFound { display_id } => {
                write!(f, "display '{display_id}' not found")
            }
            DiscoveryError::VcpReadFailed { detail } => {
                write!(f, "VCP input-source read failed: {detail}")
            }
            DiscoveryError::VcpWriteFailed { detail } => {
                write!(f, "VCP input-source write failed: {detail}")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Whether native discovery can work at all on this build.
pub fn native_available() -> bool {
    cfg!(target_os = "windows")
}

/// Enumerates displays addressable for native DDC. Empty (not an error) on non-Windows,
/// mirroring `list_native_display_ids()`.
pub fn list_displays() -> Result<Vec<DiscoveredDisplay>, DiscoveryError> {
    if !native_available() {
        return Ok(Vec::new());
    }
    list_displays_with(controller())
}

/// Reads the current VCP 0x60 input-source value for `display_id`, retrying once on read
/// failure with a fresh enumeration — real hardware showed intermittent
/// `GetVCPFeatureAndVCPFeatureReply` failures after hotplug that a refreshed lookup resolves.
pub fn read_input_source(display_id: &str) -> Result<InputSourceReading, DiscoveryError> {
    if !native_available() {
        return Err(DiscoveryError::NativeUnavailable);
    }
    read_input_source_with(controller(), display_id)
}

/// Executes one explicit VCP 0x60 write for setup-time test switching.
/// No automatic write retries: one request maps to one hardware write attempt.
/// After a successful write, one best-effort read-back is attempted; read-back failure does not
/// make probe fail.
pub fn probe_input(display_id: &str, value: u16) -> Result<ProbeInputResult, DiscoveryError> {
    if !native_available() {
        return Err(DiscoveryError::NativeUnavailable);
    }
    probe_input_with(controller(), display_id, value)
}

fn controller() -> &'static dyn NativeDdcController {
    #[cfg(target_os = "windows")]
    {
        &super::windows_ddc::WindowsDdcController
    }
    #[cfg(not(target_os = "windows"))]
    {
        &UnavailableController
    }
}

/// Trait-typed stand-in for platforms without a native controller. `list_displays`/
/// `read_input_source` short-circuit before reaching it; it exists so `controller()`
/// typechecks on every platform.
#[cfg(not(target_os = "windows"))]
struct UnavailableController;

#[cfg(not(target_os = "windows"))]
impl NativeDdcController for UnavailableController {
    fn list_displays(&self) -> io::Result<Vec<NativeDisplay>> {
        Ok(Vec::new())
    }

    fn set_vcp_feature(&self, _display_id: &str, _vcp_code: u8, _value: u16) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native DDC is not available on this platform",
        ))
    }

    fn get_vcp_feature(&self, _display_id: &str, _vcp_code: u8) -> io::Result<VcpReading> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native DDC is not available on this platform",
        ))
    }
}

fn list_displays_with(
    controller: &dyn NativeDdcController,
) -> Result<Vec<DiscoveredDisplay>, DiscoveryError> {
    controller
        .list_displays()
        .map(|displays| {
            displays
                .into_iter()
                .map(|d| DiscoveredDisplay {
                    display_id: d.display_id,
                })
                .collect()
        })
        .map_err(|e| DiscoveryError::EnumerationFailed {
            detail: e.to_string(),
        })
}

fn read_input_source_with(
    controller: &dyn NativeDdcController,
    display_id: &str,
) -> Result<InputSourceReading, DiscoveryError> {
    match attempt_read(controller, display_id) {
        Err(DiscoveryError::VcpReadFailed { detail: first }) => {
            // Retry exactly once. The controller re-enumerates per call, so this attempt runs
            // against refreshed display/physical-monitor handles — the fix for the stale-handle
            // failures observed on real hardware after hotplug. Not-found is never retried.
            match attempt_read(controller, display_id) {
                Err(DiscoveryError::VcpReadFailed { detail: second }) => {
                    Err(DiscoveryError::VcpReadFailed {
                        detail: format!("{first}; after refresh: {second}"),
                    })
                }
                other => other,
            }
        }
        other => other,
    }
}

fn probe_input_with(
    controller: &dyn NativeDdcController,
    display_id: &str,
    value: u16,
) -> Result<ProbeInputResult, DiscoveryError> {
    controller
        .set_vcp_feature(display_id, VCP_INPUT_SOURCE, value)
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                DiscoveryError::DisplayNotFound {
                    display_id: display_id.to_string(),
                }
            } else {
                DiscoveryError::VcpWriteFailed {
                    detail: e.to_string(),
                }
            }
        })?;

    let current = controller
        .get_vcp_feature(display_id, VCP_INPUT_SOURCE)
        .ok()
        .map(|r| r.current);

    Ok(ProbeInputResult {
        accepted: true,
        current,
    })
}

fn attempt_read(
    controller: &dyn NativeDdcController,
    display_id: &str,
) -> Result<InputSourceReading, DiscoveryError> {
    controller
        .get_vcp_feature(display_id, VCP_INPUT_SOURCE)
        .map(|r| InputSourceReading {
            current: r.current,
            maximum: r.maximum,
        })
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                DiscoveryError::DisplayNotFound {
                    display_id: display_id.to_string(),
                }
            } else {
                DiscoveryError::VcpReadFailed {
                    detail: e.to_string(),
                }
            }
        })
}

#[cfg(test)]
mod tests {
    use super::super::native::{NativeDisplay, VcpReading};
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct ScriptedController {
        displays: Vec<String>,
        list_error: Option<String>,
        set_vcp_results: RefCell<VecDeque<io::Result<()>>>,
        set_vcp_calls: RefCell<usize>,
        get_vcp_results: RefCell<VecDeque<io::Result<VcpReading>>>,
        get_vcp_calls: RefCell<usize>,
    }

    impl ScriptedController {
        fn new(
            displays: Vec<&str>,
            set_vcp_results: Vec<io::Result<()>>,
            get_vcp_results: Vec<io::Result<VcpReading>>,
        ) -> Self {
            Self {
                displays: displays.into_iter().map(str::to_string).collect(),
                list_error: None,
                set_vcp_results: RefCell::new(set_vcp_results.into()),
                set_vcp_calls: RefCell::new(0),
                get_vcp_results: RefCell::new(get_vcp_results.into()),
                get_vcp_calls: RefCell::new(0),
            }
        }

        fn failing_list(detail: &str) -> Self {
            Self {
                displays: Vec::new(),
                list_error: Some(detail.to_string()),
                set_vcp_results: RefCell::new(VecDeque::new()),
                set_vcp_calls: RefCell::new(0),
                get_vcp_results: RefCell::new(VecDeque::new()),
                get_vcp_calls: RefCell::new(0),
            }
        }

        fn set_calls(&self) -> usize {
            *self.set_vcp_calls.borrow()
        }

        fn calls(&self) -> usize {
            *self.get_vcp_calls.borrow()
        }
    }

    impl NativeDdcController for ScriptedController {
        fn list_displays(&self) -> io::Result<Vec<NativeDisplay>> {
            if let Some(detail) = &self.list_error {
                return Err(io::Error::other(detail.clone()));
            }
            Ok(self
                .displays
                .iter()
                .map(|id| NativeDisplay {
                    display_id: id.clone(),
                })
                .collect())
        }

        fn set_vcp_feature(&self, _display_id: &str, vcp_code: u8, _value: u16) -> io::Result<()> {
            assert_eq!(vcp_code, VCP_INPUT_SOURCE, "probe only writes VCP 0x60");
            *self.set_vcp_calls.borrow_mut() += 1;
            self.set_vcp_results
                .borrow_mut()
                .pop_front()
                .expect("ScriptedController ran out of queued set_vcp results")
        }

        fn get_vcp_feature(&self, _display_id: &str, vcp_code: u8) -> io::Result<VcpReading> {
            assert_eq!(vcp_code, VCP_INPUT_SOURCE, "discovery only reads VCP 0x60");
            *self.get_vcp_calls.borrow_mut() += 1;
            self.get_vcp_results
                .borrow_mut()
                .pop_front()
                .expect("ScriptedController ran out of queued get_vcp results")
        }
    }

    fn reading(current: u32, maximum: u32) -> io::Result<VcpReading> {
        Ok(VcpReading { current, maximum })
    }

    fn not_found(display_id: &str) -> io::Result<VcpReading> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("display '{display_id}' not found"),
        ))
    }

    fn read_failure(detail: &str) -> io::Result<VcpReading> {
        Err(io::Error::other(detail.to_string()))
    }

    #[test]
    fn reads_current_input_on_first_attempt() {
        let controller =
            ScriptedController::new(vec!["K@P:d0e5:0"], vec![], vec![reading(4626, 4626)]);

        let result = read_input_source_with(&controller, "K@P:d0e5:0").expect("should read");

        assert_eq!(
            result,
            InputSourceReading {
                current: 4626,
                maximum: 4626
            }
        );
        assert_eq!(controller.calls(), 1);
    }

    /// The real-hardware failure mode: enumeration finds the display but the VCP read fails
    /// transiently (stale handles after hotplug). One refreshed retry must recover it.
    #[test]
    fn retries_once_after_transient_read_failure() {
        let controller = ScriptedController::new(
            vec!["KJL:0e25:2"],
            vec![],
            vec![
                read_failure("no physical monitor responded"),
                reading(3, 18),
            ],
        );

        let result = read_input_source_with(&controller, "KJL:0e25:2").expect("retry should win");

        assert_eq!(
            result,
            InputSourceReading {
                current: 3,
                maximum: 18
            }
        );
        assert_eq!(controller.calls(), 2);
    }

    #[test]
    fn second_failure_reports_both_attempts_and_stops() {
        let controller = ScriptedController::new(
            vec!["KJL:0e25:2"],
            vec![],
            vec![
                read_failure("first failure"),
                read_failure("second failure"),
            ],
        );

        let err = read_input_source_with(&controller, "KJL:0e25:2").expect_err("should fail");

        assert_eq!(controller.calls(), 2);
        match err {
            DiscoveryError::VcpReadFailed { detail } => {
                assert!(detail.contains("first failure"));
                assert!(detail.contains("after refresh"));
                assert!(detail.contains("second failure"));
            }
            other => panic!("expected VcpReadFailed, got {other:?}"),
        }
    }

    #[test]
    fn missing_display_is_not_found_and_never_retried() {
        let controller =
            ScriptedController::new(vec!["OTHER:0"], vec![], vec![not_found("GHOST:0000:0")]);

        let err = read_input_source_with(&controller, "GHOST:0000:0").expect_err("should fail");

        assert_eq!(controller.calls(), 1);
        assert_eq!(
            err,
            DiscoveryError::DisplayNotFound {
                display_id: "GHOST:0000:0".to_string(),
            }
        );
        assert_eq!(err.code(), "displayNotFound");
    }

    #[test]
    fn list_maps_discovered_displays() {
        let controller = ScriptedController::new(vec!["K@P:d0e5:0", "KJL:0e25:2"], vec![], vec![]);

        let displays = list_displays_with(&controller).expect("should list");

        assert_eq!(
            displays,
            vec![
                DiscoveredDisplay {
                    display_id: "K@P:d0e5:0".to_string(),
                },
                DiscoveredDisplay {
                    display_id: "KJL:0e25:2".to_string(),
                },
            ]
        );
    }

    #[test]
    fn list_failure_is_enumeration_failed() {
        let controller = ScriptedController::failing_list("query display config failed");

        let err = list_displays_with(&controller).expect_err("should fail");

        assert_eq!(err.code(), "enumerationFailed");
        assert!(err.to_string().contains("query display config failed"));
    }

    #[test]
    fn probe_input_writes_once_and_returns_readback_when_available() {
        let controller =
            ScriptedController::new(vec!["K@P:d0e5:0"], vec![Ok(())], vec![reading(4626, 4626)]);

        let result = probe_input_with(&controller, "K@P:d0e5:0", 4626).expect("probe should work");

        assert!(result.accepted);
        assert_eq!(result.current, Some(4626));
        assert_eq!(controller.set_calls(), 1);
        assert_eq!(controller.calls(), 1);
    }

    #[test]
    fn probe_input_best_effort_readback_does_not_fail_successful_write() {
        let controller = ScriptedController::new(
            vec!["K@P:d0e5:0"],
            vec![Ok(())],
            vec![read_failure("readback failed")],
        );

        let result = probe_input_with(&controller, "K@P:d0e5:0", 4626).expect("probe should work");

        assert!(result.accepted);
        assert_eq!(result.current, None);
        assert_eq!(controller.set_calls(), 1);
        assert_eq!(controller.calls(), 1);
    }

    #[test]
    fn probe_input_missing_display_maps_to_not_found() {
        let controller = ScriptedController::new(
            vec!["OTHER:0"],
            vec![Err(io::Error::new(
                io::ErrorKind::NotFound,
                "display 'K@P:d0e5:0' not found",
            ))],
            vec![],
        );

        let err = probe_input_with(&controller, "K@P:d0e5:0", 4626).expect_err("should fail");
        assert_eq!(controller.set_calls(), 1);
        assert_eq!(err.code(), "displayNotFound");
    }

    #[test]
    fn probe_input_write_failure_maps_to_vcp_write_failed() {
        let controller = ScriptedController::new(
            vec!["K@P:d0e5:0"],
            vec![Err(io::Error::other("monitor rejected write"))],
            vec![],
        );

        let err = probe_input_with(&controller, "K@P:d0e5:0", 4626).expect_err("should fail");
        assert_eq!(controller.set_calls(), 1);
        assert_eq!(err.code(), "vcpWriteFailed");
    }

    /// Shipped non-Windows behavior of the public API: honest unavailability, not silent
    /// success. Runs on macOS/Linux CI; on Windows the public fns hit real hardware, which
    /// tests can't assume.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn public_api_is_honest_off_windows() {
        assert!(!native_available());
        assert_eq!(list_displays().expect("empty, not an error"), Vec::new());
        assert_eq!(
            read_input_source("K@P:d0e5:0").expect_err("should be unavailable"),
            DiscoveryError::NativeUnavailable
        );
        assert_eq!(
            probe_input("K@P:d0e5:0", 4626).expect_err("should be unavailable"),
            DiscoveryError::NativeUnavailable
        );
    }
}
