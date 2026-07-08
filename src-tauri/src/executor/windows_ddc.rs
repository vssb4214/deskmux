//! Real Windows implementation of [`super::native::NativeDdcController`]. Enumerates displays
//! via the Monitor Configuration API for physical-monitor handles (needed for
//! GetVCPFeature/SetVCPFeature), correlated with QueryDisplayConfig for EDID-derived identity
//! (manufacturer id + product code + connector instance) - see docs/CONFIG.md for why EDID
//! over enumeration order, and its known limitation with identical monitor models.
//!
//! Compile-checked on this Windows dev machine; real-hardware behavior is manual verification,
//! not covered by CI (see item 4 in the native-DDC plan).

use std::io;

use windows::core::BOOL;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
    GetVCPFeatureAndVCPFeatureReply, QueryDisplayConfig, SetVCPFeature,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, PHYSICAL_MONITOR,
    QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::{GetLastError, HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

use super::native::{NativeDdcController, NativeDdcFeature, NativeDisplay, VcpReading};

pub(super) struct WindowsDdcController;

impl NativeDdcController for WindowsDdcController {
    fn list_displays(&self) -> io::Result<Vec<NativeDisplay>> {
        list_displays()
            .map(|found| {
                found
                    .into_iter()
                    .map(|d| NativeDisplay {
                        display_id: d.display_id,
                    })
                    .collect()
            })
            .map_err(to_io_error)
    }

    fn set_vcp_feature(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
        value: u16,
    ) -> io::Result<()> {
        let target = find_display(display_id)?;
        let vcp_code = vcp_code(feature);
        with_physical_monitor(target.gdi_device_name, |handle| {
            (unsafe { SetVCPFeature(handle, vcp_code, u32::from(value)) } != 0).then_some(())
        })
        .map_err(to_io_error)
    }

    fn get_vcp_feature(
        &self,
        display_id: &str,
        feature: NativeDdcFeature,
    ) -> io::Result<VcpReading> {
        let target = find_display(display_id)?;
        let vcp_code = vcp_code(feature);
        with_physical_monitor(target.gdi_device_name, |handle| {
            let mut current = 0u32;
            let mut maximum = 0u32;
            let ok = unsafe {
                GetVCPFeatureAndVCPFeatureReply(
                    handle,
                    vcp_code,
                    None,
                    &mut current,
                    Some(&mut maximum),
                )
            };
            (ok != 0).then_some(VcpReading { current, maximum })
        })
        .map_err(to_io_error)
    }
}

fn vcp_code(feature: NativeDdcFeature) -> u8 {
    match feature {
        NativeDdcFeature::Brightness => 0x10,
        NativeDdcFeature::Contrast => 0x12,
        NativeDdcFeature::InputSource => 0x60,
        NativeDdcFeature::Volume => 0x62,
    }
}

/// Re-enumerates and returns the display matching `display_id`, as a fresh lookup per call —
/// deliberate, so callers retrying after a hotplug get refreshed handles rather than stale ones.
fn find_display(display_id: &str) -> io::Result<FoundDisplay> {
    list_displays()
        .map_err(to_io_error)?
        .into_iter()
        .find(|d| d.display_id == display_id)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("display '{display_id}' not found"),
            )
        })
}

fn to_io_error(err: windows::core::Error) -> io::Error {
    io::Error::other(err.to_string())
}

fn last_error() -> windows::core::Error {
    windows::core::Error::from_hresult(unsafe { GetLastError() }.to_hresult())
}

/// A display found during enumeration, keeping the GDI device name so `set_vcp_feature` can
/// re-find the same physical monitor handle without threading `HMONITOR` (not `Send`) through
/// the `NativeDdcController` trait boundary.
struct FoundDisplay {
    display_id: String,
    gdi_device_name: [u16; 32],
}

fn list_displays() -> windows::core::Result<Vec<FoundDisplay>> {
    let mut displays = Vec::new();
    for hmonitor in enum_display_monitors()? {
        let Some(gdi_device_name) = monitor_device_name(hmonitor) else {
            continue;
        };
        let Some(target) = target_device_name(gdi_device_name) else {
            continue;
        };
        displays.push(FoundDisplay {
            display_id: format!(
                "{}:{:04x}:{}",
                manufacturer_id_to_pnp_string(target.edidManufactureId),
                target.edidProductCodeId,
                target.connectorInstance
            ),
            gdi_device_name,
        });
    }
    Ok(displays)
}

/// Encodes an EDID manufacturer id (three 5-bit letters packed into a u16, per the VESA EDID
/// spec) as its three-letter PNP id string, e.g. `0x4c1a` -> `"DEL"`.
fn manufacturer_id_to_pnp_string(id: u16) -> String {
    let c1 = ((id >> 10) & 0x1f) as u8 + b'A' - 1;
    let c2 = ((id >> 5) & 0x1f) as u8 + b'A' - 1;
    let c3 = (id & 0x1f) as u8 + b'A' - 1;
    String::from_utf8_lossy(&[c1, c2, c3]).into_owned()
}

fn enum_display_monitors() -> windows::core::Result<Vec<HMONITOR>> {
    let mut monitors: Vec<HMONITOR> = Vec::new();

    unsafe extern "system" fn callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(data.0 as *mut Vec<HMONITOR>);
        monitors.push(hmonitor);
        BOOL::from(true)
    }

    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM(std::ptr::addr_of_mut!(monitors) as isize),
        )
    };
    if !ok.as_bool() {
        return Err(last_error());
    }
    Ok(monitors)
}

fn monitor_device_name(hmonitor: HMONITOR) -> Option<[u16; 32]> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let ok = unsafe { GetMonitorInfoW(hmonitor, &mut info.monitorInfo) };
    ok.as_bool().then_some(info.szDevice)
}

fn target_device_name(gdi_device_name: [u16; 32]) -> Option<DISPLAYCONFIG_TARGET_DEVICE_NAME> {
    let paths = query_display_config().ok()?;
    for path in paths {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };
        let status = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
        if status != 0 {
            continue;
        }
        if wide_str_eq(&source.viewGdiDeviceName, &gdi_device_name) {
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };
            let status = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
            if status == 0 {
                return Some(target);
            }
        }
    }
    None
}

fn wide_str_eq(a: &[u16], b: &[u16]) -> bool {
    let a_len = a.iter().position(|&c| c == 0).unwrap_or(a.len());
    let b_len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    a[..a_len] == b[..b_len]
}

fn query_display_config() -> windows::core::Result<Vec<DISPLAYCONFIG_PATH_INFO>> {
    loop {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        }
        .ok()?;

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        let result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if result == windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        result.ok()?;
        paths.truncate(path_count as usize);
        return Ok(paths);
    }
}

/// Calls `f` with the physical monitor handle for the display at `gdi_device_name`, trying each
/// physical monitor until `f` returns `Some`, cleaning up via `DestroyPhysicalMonitors`
/// regardless of the call's outcome. `f` returns `Option<T>` rather than `BOOL` so the same
/// helper serves both writes (`Option<()>`) and value-producing reads.
fn with_physical_monitor<T, F>(gdi_device_name: [u16; 32], f: F) -> windows::core::Result<T>
where
    F: Fn(HANDLE) -> Option<T>,
{
    let hmonitor = enum_display_monitors()?
        .into_iter()
        .find(|m| monitor_device_name(*m) == Some(gdi_device_name))
        .ok_or_else(last_error)?;

    let mut count = 0u32;
    unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) }?;

    let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); count as usize];
    unsafe { GetPhysicalMonitorsFromHMONITOR(hmonitor, &mut physical_monitors) }?;

    let result = physical_monitors
        .iter()
        .find_map(|pm| f(pm.hPhysicalMonitor))
        .ok_or_else(last_error);

    unsafe {
        let _ = DestroyPhysicalMonitors(&physical_monitors);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_named_features_to_vcp_codes() {
        assert_eq!(vcp_code(NativeDdcFeature::Brightness), 0x10);
        assert_eq!(vcp_code(NativeDdcFeature::Contrast), 0x12);
        assert_eq!(vcp_code(NativeDdcFeature::InputSource), 0x60);
        assert_eq!(vcp_code(NativeDdcFeature::Volume), 0x62);
    }
}
