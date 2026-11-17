//! Telemetry trackers. Each produces `TrackingEvent`s dispatched to the shared engine.

pub mod audio;
pub mod battery;
pub mod browser;
pub mod fullscreen;
pub mod input;
pub mod media;
pub mod monitor;
pub mod network;
pub mod power;
pub mod project;
pub mod uia;
pub mod window;

use crate::events::TrackingEvent;

/// Dispatch an event to the shared engine and log resulting transitions.
/// Recovers from a poisoned mutex so a panic in one tracker never silently drops
/// future telemetry (C1).
pub fn dispatch(ev: TrackingEvent) {
    if let Some(engine) = crate::STATE.get() {
        let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
        for t in guard.handle(ev) {
            crate::log::transition(&t);
        }
    }
}
