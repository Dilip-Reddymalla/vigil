//! Structured logging helpers.
//!
//! Phase 1 writes machine-readable JSON lines to stdout — the "transition log".
//! A future phase will add an optional file sink and rotation.

use crate::state::Transition;

/// Emit a transition as a single JSON line.
pub fn transition(t: &Transition) {
    // `serde_json::to_string` cannot fail for `Transition`.
    println!("{}", serde_json::to_string(t).expect("serialize transition"));
}

/// Emit an arbitrary JSON value as a line (banners, status, errors).
pub fn raw(value: &serde_json::Value) {
    println!("{}", serde_json::to_string(value).expect("serialize json"));
}
