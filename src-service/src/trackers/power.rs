//! Power & session tracker.
//!
//! Creates a hidden message-only window and registers for:
//! - `WTSRegisterSessionNotification` → lock/unlock (`WM_WTSSESSION_CHANGE`)
//! - `RegisterPowerSettingNotification(GUID_MONITOR_POWER_ON)` → monitor on/off
//! - Power broadcasts → suspend/resume (`WM_POWERBROADCAST`)
//!
//! The hidden window's window-proc dispatches `TrackingEvent`s into the shared state.
//! This function must run on a thread with a message loop (the main thread).

use crate::config::Config;
use crate::events::TrackingEvent;
use chrono::Utc;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::RegisterPowerSettingNotification;
use windows_sys::Win32::System::RemoteDesktop::WTSRegisterSessionNotification;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DEVICE_NOTIFY_WINDOW_HANDLE, DispatchMessageW,
    GetMessageW, PostQuitMessage, RegisterClassExW, TranslateMessage, MSG, WNDCLASSEXW,
    CS_HREDRAW, CS_VREDRAW,
};

// ---- Private message / constant values (Win32) ----
const WM_DESTROY: u32 = 0x0002;
const WM_POWERBROADCAST: u32 = 0x0218;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;

const WTS_SESSION_LOCK: usize = 0x7;
const WTS_SESSION_UNLOCK: usize = 0x8;
const WTS_NOTIFY_FOR_THIS_SESSION: u32 = 0x0000_0000;

const PBT_APMSUSPEND: usize = 0x0004;
const PBT_APMRESUMESUSPEND: usize = 0x0007;
const PBT_POWERSETTINGCHANGE: usize = 0x8013;

/// GUID_MONITOR_POWER_ON = {02731015-4510-4526-99E6-E5A17EBD1AEA}
const GUID_MONITOR_POWER_ON: GUID = GUID {
    data1: 0x0273_1015,
    data2: 0x4510,
    data3: 0x4526,
    data4: [0x99, 0xE6, 0xE5, 0xA1, 0x7E, 0xBD, 0x1A, 0xEA],
};

/// Mirror of `POWERBROADCAST_SETTING` (winuser.h).
#[repr(C)]
struct PowerBroadcastSetting {
    power_setting: GUID,
    data_length: u32,
    data: [u8; 1],
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == crate::tray::TRAY_CALLBACK_MSG {
        crate::tray::handle_tray_message(hwnd, lparam);
        return 0;
    }
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_WTSSESSION_CHANGE => {
            match wparam {
                WTS_SESSION_LOCK => crate::trackers::dispatch(TrackingEvent::SessionLock {
                    timestamp: Utc::now(),
                }),
                WTS_SESSION_UNLOCK => crate::trackers::dispatch(TrackingEvent::SessionUnlock {
                    timestamp: Utc::now(),
                }),
                _ => {}
            }
            0
        }
        WM_POWERBROADCAST => {
            match wparam {
                PBT_APMSUSPEND => crate::trackers::dispatch(TrackingEvent::PowerSuspend {
                    timestamp: Utc::now(),
                }),
                PBT_APMRESUMESUSPEND => crate::trackers::dispatch(TrackingEvent::PowerResume {
                    timestamp: Utc::now(),
                }),
                PBT_POWERSETTINGCHANGE => {
                    let setting = lparam as *const PowerBroadcastSetting;
                    if !setting.is_null() {
                        let on = !unsafe { (*setting).data }.is_empty()
                            && unsafe { (*setting).data[0] } != 0;
                        if on {
                            crate::trackers::dispatch(TrackingEvent::MonitorOn {
                                timestamp: Utc::now(),
                            });
                        } else {
                            crate::trackers::dispatch(TrackingEvent::MonitorOff {
                                timestamp: Utc::now(),
                            });
                        }
                    }
                }
                _ => {}
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Create the hidden window and register power/session notifications.
fn create_hidden_window(cfg: &Config) -> anyhow::Result<HWND> {
    // SAFETY: WNDCLASSEXW is fully initialized before use.
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        anyhow::bail!("GetModuleHandleW returned null instance");
    }

    let class_name: Vec<u16> = cfg.window_class.encode_utf16().chain(Some(0)).collect();

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };

    // SAFETY: registers the class described by `wc`.
    let atom = unsafe { RegisterClassExW(&wc) };
    if atom == 0 {
        anyhow::bail!("RegisterClassExW failed for window class '{}'", cfg.window_class);
    }

    // SAFETY: creates a hidden toplevel window with the registered class.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        )
    };

    if hwnd.is_null() {
        anyhow::bail!("CreateWindowExW returned a null handle");
    }

    // SAFETY: registers this window for session-change notifications.
    unsafe {
        WTSRegisterSessionNotification(hwnd, WTS_NOTIFY_FOR_THIS_SESSION);
    }
    // SAFETY: registers this window for monitor power notifications.
    unsafe {
        RegisterPowerSettingNotification(hwnd, &GUID_MONITOR_POWER_ON, DEVICE_NOTIFY_WINDOW_HANDLE);
    }

    Ok(hwnd)
}

/// Run the hidden-window message loop until quit. Blocks.
pub fn run_message_loop(cfg: &Config) -> anyhow::Result<()> {
    let hwnd = create_hidden_window(cfg)?;

    // Add the tray icon (rum before the message loop so callbacks are routed).
    if !crate::tray::add(hwnd) {
        crate::log::raw(&serde_json::json!({ "event": "error", "op": "tray_add" }));
    }

    // SAFETY: MSG is a plain POD; zero-initialize before GetMessageW fills it.
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        match result {
            0 => break, // WM_QUIT
            -1 => anyhow::bail!("GetMessageW failed"),
            _ => {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    crate::tray::remove(hwnd);
    Ok(())
}
