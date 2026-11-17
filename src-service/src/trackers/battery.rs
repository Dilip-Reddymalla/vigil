//! Battery status polling (Phase 9).
//!
//! Reads `GetSystemPowerStatus`. Returns `None` when the call fails or the
//! machine reports an unknown charge.

use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// Current battery state, if determinable.
/// `(ac_line_status, battery_life_percent, battery_flag)` — see
/// `SYSTEM_POWER_STATUS`. `ac_line_status` is 0 = on battery, 1 = on AC.
pub fn battery_status() -> Option<(u8, u8, u8)> {
    // SAFETY: SYSTEM_POWER_STATUS is a plain POD struct; we pass a valid pointer.
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut s) };
    if ok == 0 {
        return None;
    }
    // 0 = unknown. If we can't read a percentage, say so.
    if s.BatteryLifePercent == 0 && s.BatteryFlag == 0 {
        return None;
    }
    Some((s.ACLineStatus, s.BatteryLifePercent, s.BatteryFlag))
}
