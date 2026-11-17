//! **DisplayState** — monitor-power dimension: `On ⇄ Off`.
//!
//! Driven by `RegisterPowerSettingNotification(GUID_MONITOR_POWER_ON)`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayState {
    On,
    Off,
}

impl DisplayState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisplayState::On => "on",
            DisplayState::Off => "off",
        }
    }

    pub fn is_on(&self) -> bool {
        matches!(self, DisplayState::On)
    }
}

impl Default for DisplayState {
    fn default() -> Self {
        DisplayState::On
    }
}
