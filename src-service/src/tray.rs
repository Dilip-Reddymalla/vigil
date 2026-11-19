//! System tray icon (Shell_NotifyIcon) integrated with the hidden message window.
//!
//! Right-click shows a small menu: Open Dashboard / Exit. Double-click opens the
//! dashboard (Phase 5) when its binary is present beside the service.

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, LoadImageW, PostQuitMessage,
    TrackPopupMenu, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE,
    MF_SEPARATOR, MF_STRING, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_USER,
};

pub const TRAY_CALLBACK_MSG: u32 = WM_USER + 1;

const TRAY_ID: u32 = 1;
const ID_OPEN: i32 = 1001;
const ID_EXIT: i32 = 1002;

fn widen(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn set_tip(tip: &mut [u16; 128], text: &str) {
    for (i, u) in text.encode_utf16().take(127).enumerate() {
        tip[i] = u;
    }
}

/// Load the Vigil brand icon from `icons/icon.ico` beside the exe; fall back to the
/// standard application icon if missing.
fn load_app_icon() -> HICON {
    unsafe {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let ico = dir.join("icons").join("icon.ico");
                if ico.exists() {
                    let wide: Vec<u16> = ico.to_string_lossy().encode_utf16().chain(Some(0)).collect();
                    let h = LoadImageW(
                        std::ptr::null_mut(),
                        wide.as_ptr(),
                        IMAGE_ICON,
                        0,
                        0,
                        LR_LOADFROMFILE | LR_DEFAULTSIZE,
                    );
                    if !h.is_null() {
                        return h as HICON;
                    }
                }
            }
        }
        LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
    }
}

/// Add the tray icon for the given window.
pub fn add(hwnd: HWND) -> bool {
    // SAFETY: icon load + tray registration.
    let icon = load_app_icon();
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = TRAY_CALLBACK_MSG;
    nid.hIcon = icon;
    set_tip(&mut nid.szTip, "Vigil");
    unsafe { Shell_NotifyIconW(NIM_ADD, &nid) != 0 }
}

/// Remove the tray icon.
pub fn remove(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// Handle a tray callback message (from the hidden-window proc).
pub fn handle_tray_message(hwnd: HWND, lparam: isize) {
    match lparam as u32 {
        WM_LBUTTONDBLCLK => open_dashboard(),
        WM_RBUTTONUP => show_menu(hwnd),
        _ => {}
    }
}

fn show_menu(hwnd: HWND) {
    // SAFETY: create/show/destroy a popup menu; TPM_RETURNCMD returns the selection.
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, ID_OPEN as usize, widen("Open Dashboard").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, ID_EXIT as usize, widen("Exit").as_ptr());

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);

        let cmd = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_RETURNCMD,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);

        match cmd {
            ID_EXIT => {
                crate::log::raw(&serde_json::json!({ "event": "tray_exit" }));
                PostQuitMessage(0);
            }
            ID_OPEN => open_dashboard(),
            _ => {}
        }
    }
}

/// Launch the dashboard. Preferred: a `dashboard-launcher.cmd` override beside the
/// service (used on Smart App Control systems, where the compiled dashboard exe is
/// blocked and we instead open a Python-served browser dashboard). Falls back to
/// launching `vigil-dashboard.exe` if present.
fn open_dashboard() {
    let Some(exe) = std::env::current_exe().ok() else {
        return;
    };
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));

    // Override launcher (python-dashboard mode).
    let launcher = dir.join("dashboard-launcher.cmd");
    if launcher.exists() {
        match std::process::Command::new("cmd.exe").arg("/C").arg(&launcher).spawn() {
            Ok(_) => crate::log::raw(&serde_json::json!({
                "event": "open_dashboard", "status": "launcher_spawned"
            })),
            Err(e) => crate::log::raw(&serde_json::json!({
                "event": "open_dashboard", "status": "launcher_error", "error": e.to_string()
            })),
        }
        return;
    }

    // Default: launch the dashboard binary if present next to this executable.
    let dashboard = dir.join("vigil-dashboard.exe");
    if !dashboard.exists() {
        crate::log::raw(&serde_json::json!({
            "event": "open_dashboard", "status": "not_found"
        }));
        return;
    }
    match std::process::Command::new(dashboard).spawn() {
        Ok(_) => crate::log::raw(&serde_json::json!({
            "event": "open_dashboard", "status": "spawned"
        })),
        Err(e) => crate::log::raw(&serde_json::json!({
            "event": "open_dashboard", "status": "error", "error": e.to_string()
        })),
    }
}
