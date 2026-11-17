//! Foreground-window tracker.
//!
//! - Hooks `EVENT_SYSTEM_FOREGROUND` (window switches) AND
//!   `EVENT_OBJECT_NAMECHANGE` (title/navigation changes in the active window).
//! - Seeds the current foreground window on install (GetForegroundWindow) so we
//!   don't wait for the first switch (C9).
//! - Provides `poll_foreground()` for the input poller to re-check the active
//!   window title (C10 fallback).

use crate::events::TrackingEvent;
use chrono::Utc;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HWND};
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, EVENT_OBJECT_NAMECHANGE,
    EVENT_SYSTEM_FOREGROUND,
};

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const OBJID_WINDOW: i32 = 0x0000_0000;
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000_0000;

fn process_name(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    // Primary: QueryFullProcessImageNameW.
    let via_query = unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            None
        } else {
            let mut buf = [0u16; 1024];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);
            if ok == 0 || size == 0 {
                None
            } else {
                Some(String::from_utf16_lossy(&buf[..size as usize]))
            }
        }
    };
    if let Some(path) = via_query {
        return path.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    }

    // Fallback: Toolhelp32 snapshot (works even when OpenProcess is denied).
    toolhelp_process_name(pid)
}

fn toolhelp_process_name(pid: u32) -> String {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: standard snapshot iteration; entries are POD.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return String::new();
        };
        if snapshot.0.is_null() {
            return String::new();
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = String::new();
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let len = entry.szExeFile.iter().position(|&u| u == 0).unwrap_or(entry.szExeFile.len());
                    found = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    break;
                }
                if !Process32NextW(snapshot, &mut entry).is_ok() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
        found
    }
}

/// Read a window's title.
fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

/// Dispatch a WindowChange for `hwnd` (resolves pid + title + process name).
fn dispatch_for_window(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let mut process_id: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }
    let title = window_title(hwnd);
    let process_name = process_name(process_id);

    // Browser tab enumeration (log-only, best-effort).
    if crate::trackers::browser::adapter_for_process(&process_name).is_some() {
        if let Some(tabs) = crate::trackers::uia::read_tabs(windows::Win32::Foundation::HWND(hwnd)) {
            crate::log::raw(&serde_json::json!({
                "event": "tabs",
                "process": process_name,
                "count": tabs.len(),
                "tabs": tabs,
            }));
        }
    }

    crate::trackers::dispatch(TrackingEvent::WindowChange {
        process_id,
        process_name,
        title,
        hwnd: hwnd as isize,
        timestamp: Utc::now(),
    });
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if hwnd.is_null() || id_object != OBJID_WINDOW {
        return;
    }
    match event {
        EVENT_SYSTEM_FOREGROUND => dispatch_for_window(hwnd),
        // Title/navigation change — only handle if this IS the foreground window.
        EVENT_OBJECT_NAMECHANGE => {
            let fg = GetForegroundWindow();
            if fg == hwnd {
                dispatch_for_window(hwnd);
            }
        }
        _ => {}
    }
}

/// Install the foreground-window hook and seed the current foreground window.
pub fn install() -> anyhow::Result<HWINEVENTHOOK> {
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_OBJECT_NAMECHANGE, // range covers both events
            std::ptr::null_mut(),
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_null() {
        anyhow::bail!("SetWinEventHook failed (win32 error {})", unsafe { GetLastError() });
    }

    // C9: seed the already-foreground window so we don't wait for a switch.
    let fg = unsafe { GetForegroundWindow() };
    dispatch_for_window(fg);

    Ok(hook)
}

/// Re-check the current foreground window (called on the input poll tick).
pub fn poll_foreground() {
    let fg = unsafe { GetForegroundWindow() };
    if !fg.is_null() {
        dispatch_for_window(fg);
    }
}

/// Remove the hook (call on shutdown).
pub fn uninstall(hook: HWINEVENTHOOK) {
    if !hook.is_null() {
        unsafe {
            UnhookWinEvent(hook);
        }
    }
}
