//! Runtime configuration for the Vigil service.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Config {
    /// Seconds of no input after which the user is considered `Active` → `Passive`.
    /// Same as the poll interval.
    pub active_window_seconds: u32,
    /// Seconds of no input after which the user is considered `Idle`
    /// (default 20 minutes = 1200s). Stored in a shared atomic so it can be
    /// hot-reloaded without restarting the service (M7).
    pub idle_threshold_seconds: Arc<AtomicU32>,
    /// Heartbeat period in seconds.
    pub heartbeat_seconds: u32,
    /// The hidden message-window class name.
    pub window_class: &'static str,
}

impl Config {
    /// Current idle threshold in seconds.
    pub fn idle_threshold(&self) -> u32 {
        self.idle_threshold_seconds.load(Ordering::Relaxed)
    }

    /// Update the idle threshold (affects the running poller immediately).
    pub fn set_idle_threshold(&self, seconds: u32) {
        self.idle_threshold_seconds.store(seconds, Ordering::Relaxed);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_window_seconds: 5,
            idle_threshold_seconds: Arc::new(AtomicU32::new(1200)),
            heartbeat_seconds: 30,
            window_class: "VigilHiddenWindow",
        }
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("active_window_seconds", &self.active_window_seconds)
            .field("idle_threshold_seconds", &self.idle_threshold())
            .field("heartbeat_seconds", &self.heartbeat_seconds)
            .field("window_class", &self.window_class)
            .finish()
    }
}
