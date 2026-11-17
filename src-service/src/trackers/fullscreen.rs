//! Fullscreen / presentation awareness (Phase 1).
//!
//! Queries `SHQueryUserNotificationState` to detect exclusive-D3D games,
//! fullscreen video, and presentation mode. When such a state is active, the
//! engine clamps the activity floor to `Passive` so immersive media/gaming is
//! never dropped to `Idle` just because the user isn't typing/moving the mouse.

use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUERY_USER_NOTIFICATION_STATE, QUNS_BUSY,
    QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};

/// Current Windows user-notification state (0 = not present).
pub fn user_notification_state() -> u32 {
    unsafe { SHQueryUserNotificationState() }
        .map(|s: QUERY_USER_NOTIFICATION_STATE| s.0 as u32)
        .unwrap_or(0)
}

/// True when immersive fullscreen/presentation media is running, so we should not
/// mark the user idle despite no keyboard/mouse input.
pub fn is_immersive(state: u32) -> bool {
    matches!(
        state,
        q if q == QUNS_BUSY.0 as u32
            || q == QUNS_RUNNING_D3D_FULL_SCREEN.0 as u32
            || q == QUNS_PRESENTATION_MODE.0 as u32
    )
}

/// Convenience: is the system currently in an immersive state?
pub fn immersive_now() -> bool {
    is_immersive(user_notification_state())
}

/// Whether the screen-time "shield" (fullscreen/audio) is enabled. Exposed so
/// tests can disable it for deterministic behavior.
pub static SHIELDS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[cfg(test)]
pub fn set_shields_enabled(v: bool) {
    SHIELDS_ENABLED.store(v, std::sync::atomic::Ordering::Relaxed);
}

/// Combine both shields: if enabled AND (immersive UI OR audio is playing), we
/// should not downgrade to Idle.
pub fn should_keep_active() -> bool {
    SHIELDS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
        && (immersive_now() || crate::trackers::audio::audio_playing())
}
