//! Multi-monitor attribution (Phase 8).
//!
//! Walks the monitor list and returns the ordinal index (0-based) of the monitor
//! that hosts the given window handle. This lets us attribute sessions to
//! "Display 1 / Display 2".

use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, MonitorFromWindow, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _lprc: *mut RECT,
    dw: LPARAM,
) -> BOOL {
    let list = &mut *(dw as *mut Vec<i64>);
    list.push(hmon as i64);
    1 // TRUE
}

/// Return the 0-based monitor index hosting `hwnd`, or 0 if undeterminable.
pub fn monitor_index_for_window(hwnd: isize) -> u32 {
    if hwnd == 0 {
        return 0;
    }

    // 1. Enumerate monitors → list of HMONITORs (ordinal = index).
    let mut monitors: Vec<i64> = Vec::new();
    // SAFETY: EnumDisplayMonitors calls our callback with dwData as context.
    unsafe {
        EnumDisplayMonitors(
            0 as HDC,
            std::ptr::null(),
            Some(enum_proc),
            &mut monitors as *mut Vec<i64> as LPARAM,
        );
    }

    // 2. Resolve the window's monitor.
    // SAFETY: MonitorFromWindow/GetWindowRect are stateless reads.
    let target = unsafe { MonitorFromWindow(hwnd as _, MONITOR_DEFAULTTONEAREST) } as i64;

    // 3. Find its ordinal.
    for (i, m) in monitors.iter().enumerate() {
        if *m == target {
            return i as u32;
        }
    }

    // Fallback: if the window rect is null/exclusive-fullscreen, use 0.
    let _ = unsafe { GetWindowRect(hwnd as _, std::ptr::null_mut()) };
    0
}
