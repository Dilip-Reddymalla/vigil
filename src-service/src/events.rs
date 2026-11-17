//! Canonical tracking events flowing through the Vigil engine.
//!
//! Every source of telemetry produces one of these events. The state coordinator
//! (`crate::state::MonitorState`) consumes them to update the three orthogonal
//! dimensions and to detect screen-time transitions.

use crate::state::{ActivityState, DisplayState, SystemState};

/// A single telemetry observation from one tracker.
#[derive(Debug, Clone)]
pub enum TrackingEvent {
    /// The foreground window changed (from `SetWinEventHook` or polling).
    WindowChange {
        process_id: u32,
        process_name: String,
        title: String,
        /// Raw window handle (isize) so split-screen/multi-window sessions of the
        /// same process can be distinguished.
        hwnd: isize,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Periodic input poll (every `active_window` seconds) reporting the computed
    /// activity state from `GetLastInputInfo` (active/passive/idle).
    ActivityPoll {
        state: ActivityState,
        last_input_seconds: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The workstation was locked (Win+L / screen saver).
    SessionLock {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The workstation was unlocked.
    SessionUnlock {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The machine entered sleep/suspend.
    PowerSuspend {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The machine resumed from sleep/suspend.
    PowerResume {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The primary monitor entered standby/off.
    MonitorOff {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// The primary monitor turned on.
    MonitorOn {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl TrackingEvent {
    /// The human-readable name of this event (used in transition logs).
    pub fn kind(&self) -> &'static str {
        match self {
            TrackingEvent::WindowChange { .. } => "window_change",
            TrackingEvent::ActivityPoll { .. } => "activity_poll",
            TrackingEvent::SessionLock { .. } => "session_lock",
            TrackingEvent::SessionUnlock { .. } => "session_unlock",
            TrackingEvent::PowerSuspend { .. } => "power_suspend",
            TrackingEvent::PowerResume { .. } => "power_resume",
            TrackingEvent::MonitorOff { .. } => "monitor_off",
            TrackingEvent::MonitorOn { .. } => "monitor_on",
        }
    }

    /// Resolve the target dimension value for this event, if it changes one.
    pub fn target(&self) -> Option<DimensionTarget> {
        match self {
            TrackingEvent::ActivityPoll { state, .. } => {
                Some(DimensionTarget::Activity(*state))
            }
            TrackingEvent::SessionLock { .. } => Some(DimensionTarget::System(SystemState::Locked)),
            TrackingEvent::SessionUnlock { .. } => Some(DimensionTarget::System(SystemState::Unlocked)),
            TrackingEvent::PowerSuspend { .. } => Some(DimensionTarget::System(SystemState::Sleeping)),
            // Resuming from sleep typically shows the lock screen first; the
            // subsequent WTS_SESSION_UNLOCK moves us to Unlocked. Counting lock
            // time as screen time would inflate totals.
            TrackingEvent::PowerResume { .. } => Some(DimensionTarget::System(SystemState::Locked)),
            TrackingEvent::MonitorOff { .. } => Some(DimensionTarget::Display(DisplayState::Off)),
            TrackingEvent::MonitorOn { .. } => Some(DimensionTarget::Display(DisplayState::On)),
            TrackingEvent::WindowChange { .. } => None,
        }
    }
}

/// Which dimension (and to what value) an event targets.
#[derive(Debug, Clone, Copy)]
pub enum DimensionTarget {
    System(SystemState),
    Activity(ActivityState),
    Display(DisplayState),
}
