//! **ActivityState** — user-input dimension: `Active ⇄ Passive ⇄ Idle`.
//!
//! Driven by `GetLastInputInfo()` (seconds since last keyboard/mouse event):
//! - `< active_window`   → `Active`
//! - `< idle_threshold`  → `Passive`
//! - `>= idle_threshold` → `Idle`

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityState {
    Active,
    Passive,
    Idle,
}

impl ActivityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityState::Active => "active",
            ActivityState::Passive => "passive",
            ActivityState::Idle => "idle",
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, ActivityState::Idle)
    }
}

impl Default for ActivityState {
    fn default() -> Self {
        ActivityState::Active
    }
}
