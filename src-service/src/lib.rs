//! Vigil background service library.
//!
//! Phase 1: tracking engine (three-dimensional state model, trackers, transition logs).
//! Phase 2: data correctness (SQLite writer, migrations, heartbeat, session/recovery).

pub mod categorizer;
pub mod config;
pub mod db;
pub mod engine;
pub mod events;
pub mod ipc;
pub mod lifecycle;
pub mod log;
pub mod state;
pub mod trackers;
pub mod tray;

use std::sync::{Arc, Mutex, OnceLock};

use engine::Engine;

/// Global shared engine. Set once at startup; used by all tracker callbacks.
pub static STATE: OnceLock<Arc<Mutex<Engine>>> = OnceLock::new();

/// Open the database, run startup recovery, and register the global engine.
pub fn init_engine() -> anyhow::Result<Arc<Mutex<Engine>>> {
    let db = db::Database::open()?;
    let engine = Engine::new(db)?;
    let arc = Arc::new(Mutex::new(engine));
    let _ = STATE.set(arc.clone());
    Ok(arc)
}
