//! **Coordinator** — owns the three state dimensions, applies telemetry events, and
//! produces non-overlapping transition logs.
//!
//! Screen time is counted **iff** `system == Unlocked && activity != Idle && display == On`.

use crate::events::{DimensionTarget, TrackingEvent};
use crate::state::{ActivityState, DisplayState, SystemState};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Combined screen-time classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenTimeKind {
    /// Not screen time (locked, idle, or display off).
    None,
    /// Unlocked + Active + On (user is interacting).
    Active,
    /// Unlocked + Passive + On (user is reading/watching).
    Passive,
}

impl ScreenTimeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenTimeKind::None => "none",
            ScreenTimeKind::Active => "active",
            ScreenTimeKind::Passive => "passive",
        }
    }
}

/// A single state transition to be logged (one line of the transition log).
#[derive(Debug, Clone, Serialize)]
pub struct Transition {
    pub ts: DateTime<Utc>,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: String,
    pub screen_time: bool,
}

/// Information about the current foreground window.
#[derive(Debug, Clone)]
pub struct Foreground {
    pub process_id: u32,
    pub process_name: String,
    pub title: String,
    /// Raw window handle (isize) to differentiate split-screen windows.
    pub hwnd: isize,
}

/// The full monitor state. `Arc<Mutex<MonitorState>>` is shared across tracker threads.
#[derive(Debug, Clone)]
pub struct MonitorState {
    system: SystemState,
    activity: ActivityState,
    display: DisplayState,
    foreground: Option<Foreground>,
    last_screen_kind: ScreenTimeKind,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            system: SystemState::default(),
            activity: ActivityState::default(),
            display: DisplayState::default(),
            foreground: None,
            last_screen_kind: ScreenTimeKind::None,
        }
    }
}

impl MonitorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The "effective" screen-time rule.
    pub fn is_screen_time(&self) -> bool {
        self.system.is_unlocked() && !self.activity.is_idle() && self.display.is_on()
    }

    /// Classify combined screen time.
    pub fn screen_kind(&self) -> ScreenTimeKind {
        if !self.is_screen_time() {
            return ScreenTimeKind::None;
        }
        if self.activity == ActivityState::Active {
            ScreenTimeKind::Active
        } else {
            ScreenTimeKind::Passive
        }
    }

    /// Current foreground window, if any.
    pub fn foreground(&self) -> Option<&Foreground> {
        self.foreground.as_ref()
    }

    /// The three dimensions.
    pub fn system(&self) -> SystemState {
        self.system
    }
    pub fn activity(&self) -> ActivityState {
        self.activity
    }
    pub fn display(&self) -> DisplayState {
        self.display
    }

    /// Apply a telemetry event. Returns the list of transitions to log.
    pub fn apply(&mut self, ev: TrackingEvent) -> Vec<Transition> {
        let ts = event_timestamp(&ev);
        let mut out = Vec::new();

        match ev {
            TrackingEvent::WindowChange {
                process_id,
                process_name,
                title,
                hwnd,
                ..
            } => {
                let changed = self.foreground.as_ref().map_or(true, |f| {
                    f.process_id != process_id
                        || f.process_name != process_name
                        || f.title != title
                        || f.hwnd != hwnd
                });
                self.foreground = Some(Foreground {
                    process_id,
                    process_name,
                    title: title.clone(),
                    hwnd,
                });
                if changed {
                    out.push(Transition {
                        ts,
                        kind: "window",
                        from: None,
                        to: format!("pid={process_id} win={hwnd} exe=\"{title}\""),
                        screen_time: self.is_screen_time(),
                    });
                }
            }
            other => {
                if let Some(target) = other.target() {
                    if let Some((dim, from, to)) = self.apply_target(target) {
                        out.push(Transition {
                            ts,
                            kind: dim,
                            from: Some(from),
                            to,
                            screen_time: self.is_screen_time(),
                        });
                    }
                }
            }
        }

        // Detect screen-time eligibility flips.
        let kind = self.screen_kind();
        if kind != self.last_screen_kind {
            let prev = self.last_screen_kind;
            self.last_screen_kind = kind;
            out.push(Transition {
                ts,
                kind: "screen",
                from: Some(prev.as_str().to_string()),
                to: kind.as_str().to_string(),
                screen_time: kind != ScreenTimeKind::None,
            });
        }

        out
    }

    /// Returns `None` when the dimension value did not change (avoids no-op logs).
    fn apply_target(&mut self, target: DimensionTarget) -> Option<(&'static str, String, String)> {
        match target {
            DimensionTarget::System(new) => {
                if self.system == new {
                    return None;
                }
                let from = self.system.as_str().to_string();
                self.system = new;
                Some(("system", from, new.as_str().to_string()))
            }
            DimensionTarget::Activity(new) => {
                if self.activity == new {
                    return None;
                }
                let from = self.activity.as_str().to_string();
                self.activity = new;
                Some(("activity", from, new.as_str().to_string()))
            }
            DimensionTarget::Display(new) => {
                if self.display == new {
                    return None;
                }
                let from = self.display.as_str().to_string();
                self.display = new;
                Some(("display", from, new.as_str().to_string()))
            }
        }
    }

    /// JSON snapshot of the current state (used for status/heartbeat).
    pub fn snapshot_json(&self) -> serde_json::Value {
        let fg = self.foreground.as_ref().map(|f| {
            serde_json::json!({
                "process_id": f.process_id,
                "process_name": f.process_name,
                "title": f.title,
            })
        });
        serde_json::json!({
            "system": self.system.as_str(),
            "activity": self.activity.as_str(),
            "display": self.display.as_str(),
            "screen_time": self.is_screen_time(),
            "screen_kind": self.screen_kind().as_str(),
            "foreground": fg,
        })
    }
}

fn event_timestamp(ev: &TrackingEvent) -> DateTime<Utc> {
    match ev {
        TrackingEvent::WindowChange { timestamp, .. } => *timestamp,
        TrackingEvent::ActivityPoll { timestamp, .. } => *timestamp,
        TrackingEvent::SessionLock { timestamp } => *timestamp,
        TrackingEvent::SessionUnlock { timestamp } => *timestamp,
        TrackingEvent::PowerSuspend { timestamp } => *timestamp,
        TrackingEvent::PowerResume { timestamp } => *timestamp,
        TrackingEvent::MonitorOff { timestamp } => *timestamp,
        TrackingEvent::MonitorOn { timestamp } => *timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TrackingEvent;
    use chrono::Utc;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn screen_time_requires_all_three() {
        let mut m = MonitorState::new();
        assert!(m.is_screen_time());
        assert_eq!(m.screen_kind(), ScreenTimeKind::Active);

        m.apply(TrackingEvent::SessionLock { timestamp: now() });
        assert!(!m.is_screen_time());
        m.apply(TrackingEvent::SessionUnlock { timestamp: now() });
        assert!(m.is_screen_time());

        m.apply(TrackingEvent::ActivityPoll {
            state: ActivityState::Idle,
            last_input_seconds: 400,
            timestamp: now(),
        });
        assert!(!m.is_screen_time());
        m.apply(TrackingEvent::ActivityPoll {
            state: ActivityState::Active,
            last_input_seconds: 1,
            timestamp: now(),
        });

        m.apply(TrackingEvent::MonitorOff { timestamp: now() });
        assert!(!m.is_screen_time());
        m.apply(TrackingEvent::MonitorOn { timestamp: now() });
        assert!(m.is_screen_time());
    }

    #[test]
    fn passive_reported_when_not_active() {
        let mut m = MonitorState::new();
        m.apply(TrackingEvent::ActivityPoll {
            state: ActivityState::Passive,
            last_input_seconds: 40,
            timestamp: now(),
        });
        assert!(m.is_screen_time());
        assert_eq!(m.screen_kind(), ScreenTimeKind::Passive);
    }

    #[test]
    fn unlock_resumes_screen_time() {
        let mut m = MonitorState::new();
        m.apply(TrackingEvent::SessionLock { timestamp: now() });
        assert_eq!(m.screen_kind(), ScreenTimeKind::None);
        let trs = m.apply(TrackingEvent::SessionUnlock { timestamp: now() });
        assert!(trs.iter().any(|t| t.kind == "screen" && t.to == "active"));
    }

    #[test]
    fn noop_dimension_transitions_are_suppressed() {
        let mut m = MonitorState::new();
        let trs = m.apply(TrackingEvent::MonitorOn { timestamp: now() });
        // display already `on` -> no display transition emitted
        assert!(!trs.iter().any(|t| t.kind == "display"));
    }

    #[test]
    fn foreground_updates() {
        let mut m = MonitorState::new();
        m.apply(TrackingEvent::WindowChange {
            process_id: 42,
            process_name: "code.exe".into(),
            title: "Editor".into(),
            hwnd: 0,
            timestamp: now(),
        });
        let fg = m.foreground().unwrap();
        assert_eq!(fg.process_name, "code.exe");
        assert_eq!(fg.process_id, 42);
    }
}
