//! Single-row heartbeat table (`id = 1`). Upsert only — never append.

use rusqlite::{params, Connection};

use crate::state::{ActivityState, DisplayState, SystemState};

/// Snapshot of the service's last-known state for the heartbeat.
pub struct HeartbeatSnapshot<'a> {
    pub last_alive_at: &'a str,
    pub system_state: SystemState,
    pub activity_state: ActivityState,
    pub display_state: DisplayState,
    pub active_session: Option<&'a str>,
}

/// Upsert the heartbeat row.
pub fn write(conn: &Connection, hb: &HeartbeatSnapshot) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE heartbeat
            SET last_alive_at = ?1,
                system_state   = ?2,
                activity_state = ?3,
                display_state  = ?4,
                active_session  = ?5,
                updated_at      = ?1
          WHERE id = 1",
        params![
            hb.last_alive_at,
            hb.system_state.as_str(),
            hb.activity_state.as_str(),
            hb.display_state.as_str(),
            hb.active_session
        ],
    )?;
    Ok(())
}

/// Read the heartbeat. Returns `(last_alive_at, active_session_uuid)`.
pub fn read(conn: &Connection) -> rusqlite::Result<(Option<String>, Option<String>)> {
    conn.query_row(
        "SELECT NULLIF(last_alive_at, ''), active_session FROM heartbeat WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}
