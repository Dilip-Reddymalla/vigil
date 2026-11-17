//! Per-tab UI Automation reader for Chromium browsers (native, in-service).
//!
//! Creates one `IUIAutomation` lazily and only queries it on window changes, so
//! runtime RAM impact is minimal.

use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationValuePattern, TreeScope_Subtree,
    UIA_CONTROLTYPE_ID, UIA_EditControlTypeId, UIA_TabItemControlTypeId, UIA_ValuePatternId,
};

/// Create a temporary automation object. Only called on the message-loop thread,
/// and only when a foreground window changes, so cost is negligible.
fn automation() -> windows::core::Result<IUIAutomation> {
    // SAFETY: COM init on this thread; we drop the object before returning.
    unsafe {
        let _ = CoInitializeEx(Some(std::ptr::null()), COINIT_MULTITHREADED);
    }
    unsafe {
        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
    }
}

fn is_tab_element(el: &IUIAutomationElement) -> bool {
    unsafe { el.CurrentControlType() }
        .map(|t| t == UIA_CONTROLTYPE_ID(UIA_TabItemControlTypeId.0))
        .unwrap_or(false)
}

fn element_name(el: &IUIAutomationElement) -> Option<String> {
    let name = unsafe { el.CurrentName() }.ok()?;
    let s = name.to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Enumerate browser tab titles via UI Automation.
/// Returns `None` if the accessibility tree is unavailable (e.g. browser blocks it).
pub fn read_tabs(hwnd: HWND) -> Option<Vec<String>> {
    if hwnd.0.is_null() {
        return None;
    }
    let auto = match automation() {
        Ok(a) => a,
        Err(e) => {
            crate::log::raw(&serde_json::json!({ "event": "uia", "stage": "create", "error": e.to_string() }));
            return None;
        }
    };

    let root = match unsafe { auto.ElementFromHandle(hwnd) } {
        Ok(r) => r,
        Err(e) => {
            crate::log::raw(&serde_json::json!({ "event": "uia", "stage": "handle", "error": e.to_string() }));
            return None;
        }
    };

    let true_cond = match unsafe { auto.CreateTrueCondition() } {
        Ok(c) => c,
        Err(e) => {
            crate::log::raw(&serde_json::json!({ "event": "uia", "stage": "cond", "error": e.to_string() }));
            return None;
        }
    };

    let found = match unsafe { root.FindAll(TreeScope_Subtree, &true_cond) } {
        Ok(f) => f,
        Err(e) => {
            crate::log::raw(&serde_json::json!({ "event": "uia", "stage": "findall", "error": e.to_string() }));
            return None;
        }
    };

    let length = unsafe { found.Length() }.ok()?;
    let mut tabs = Vec::new();
    for i in 0..length {
        let Ok(el) = (unsafe { found.GetElement(i) }) else { continue };
        if is_tab_element(&el) {
            if let Some(name) = element_name(&el) {
                tabs.push(name);
            }
        }
    }

    if tabs.is_empty() { None } else { Some(tabs) }
}

/// Read the browser address bar via UI Automation (Chromium exposes an Edit control
/// with the Value pattern). Falls back to `None` if unavailable (C7).
pub fn read_address_bar(hwnd: HWND) -> Option<String> {
    if hwnd.0.is_null() {
        return None;
    }
    let auto = automation().ok()?;
    let root = unsafe { auto.ElementFromHandle(hwnd) }.ok()?;

    let true_cond = unsafe { auto.CreateTrueCondition() }.ok()?;
    let found = unsafe { root.FindAll(TreeScope_Subtree, &true_cond) }.ok()?;
    let length = unsafe { found.Length() }.ok()?;
    let edit_type = UIA_CONTROLTYPE_ID(UIA_EditControlTypeId.0);

    for i in 0..length {
        let Ok(el) = (unsafe { found.GetElement(i) }) else { continue };
        // Only consider Edit controls.
        let Ok(ct) = (unsafe { el.CurrentControlType() }) else { continue };
        if ct != edit_type {
            continue;
        }
        // Try to obtain the Value pattern.
        let Ok(pattern) = (unsafe { el.GetCurrentPattern(UIA_ValuePatternId) }) else {
            continue;
        };
        let Ok(value) = pattern.cast::<IUIAutomationValuePattern>() else {
            continue;
        };
        let Ok(text) = (unsafe { value.CurrentValue() }) else { continue };
        let s = text.to_string();
        if !s.is_empty() && s.contains('.') {
            return Some(s);
        }
    }
    None
}
