//! Input tracker.
//!
//! Polls `GetLastInputInfo()` every `active_window` seconds on a dedicated thread
//! and drives the ActivityState dimension: Active → Passive → Idle.
//!
//! Semantics (see skill):
//! - seconds since last input < `active_window`  → `Active`
//! - seconds since last input < `idle_threshold` → `Passive`
//! - otherwise → `Idle`

use crate::config::Config;
use crate::events::TrackingEvent;
use crate::state::ActivityState;
use chrono::Utc;
use std::time::Duration;
use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

/// Last-seen XInput packet numbers per controller (0..=3). If a packet number
/// changes, the user pressed a button / moved a thumbstick (Phase 2).
static LAST_PACKET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// True if any connected controller has produced new input since the last poll.
fn controller_active() -> bool {
    let mut prev = LAST_PACKET.load(std::sync::atomic::Ordering::Relaxed);
    let first = prev == u64::MAX;
    let mut changed = false;
    for slot in 0..4 {
        // SAFETY: XInputGetState is a stateless read; safe to call from any thread.
        let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
        let ret = unsafe { XInputGetState(slot, &mut state) };
        if ret == 0 {
            // ERROR_SUCCESS → controller connected; packet number changes = input.
            let packet = state.dwPacketNumber as u64;
            let prev_slot = (prev >> (slot * 16)) & 0xFFFF;
            if !first && prev_slot != packet {
                changed = true;
            }
            prev = (prev & !(0xFFFFu64 << (slot * 16))) | (packet << (slot * 16));
        }
    }
    LAST_PACKET.store(prev, std::sync::atomic::Ordering::Relaxed);
    changed
}

/// Seconds since the last keyboard/mouse input (wrap-safe using u32 ticks).
fn last_input_seconds() -> u32 {
    // SAFETY: LASTINPUTINFO is a POD struct we initialize with its correct size.
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut info) };
    if ok == 0 {
        return 0;
    }
    let now = unsafe { GetTickCount() };
    now.wrapping_sub(info.dwTime) / 1000
}

/// Spawn the input poller thread.
pub fn spawn(cfg: Config) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("vigil-input".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(cfg.active_window_seconds as u64));

            let idle_threshold = cfg.idle_threshold();
            let seconds = last_input_seconds();
            // Controller input counts as Active (Phase 2: gaming).
            let target = if controller_active() || seconds < cfg.active_window_seconds {
                ActivityState::Active
            } else if seconds < idle_threshold {
                ActivityState::Passive
            } else {
                ActivityState::Idle
            };

            crate::trackers::dispatch(TrackingEvent::ActivityPoll {
                state: target,
                last_input_seconds: seconds,
                timestamp: Utc::now(),
            });
        })
        .expect("spawn input tracker thread")
}
