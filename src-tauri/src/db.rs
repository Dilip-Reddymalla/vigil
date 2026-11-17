#![cfg(feature = "tauri")]

//! Read-only SQLite access for the dashboard (M6). Heavy historical queries run
//! directly against the service's database in READONLY mode instead of blocking on
//! the named pipe. The service remains the sole writer.

use rusqlite::{Connection, OpenFlags};
use serde_json::json;

/// Path to the service database.
fn db_path() -> Option<std::path::PathBuf> {
    let appdata = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(appdata).join("Vigil").join("data.db"))
}

/// Open the database read-only.
fn open_readonly() -> Option<Connection> {
    let path = db_path()?;
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}

/// Load a daily summary straight from SQLite (read-only). Returns `None` if the
/// DB is unavailable or the day is missing.
pub fn summary(local_date: &str) -> Option<serde_json::Value> {
    let conn = open_readonly()?;
    // Prefer the rollup summary for historical days; fall back to raw sessions.
    let row = conn
        .query_row::<(i64, i64, i64, i64, i64, i64, i64, f64, i64, i64, i64, f64, i64, i64, String), _, _>(
            "SELECT total_screen_time_ms, total_active_time_ms, total_passive_time_ms,
                    total_idle_time_ms, total_locked_time_ms, total_sleep_time_ms,
                    session_count, productivity_score, deep_work_sessions, deep_work_time_ms,
                    app_switches, distraction_index, total_bytes_sent, total_bytes_received,
                    hourly_breakdown_json
               FROM daily_summaries WHERE local_date = ?1",
            [local_date],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                    r.get(12)?,
                    r.get(13)?,
                    r.get(14)?,
                ))
            },
        )
        .ok()?;

    let hourly: Vec<i64> = serde_json::from_str(&row.14).unwrap_or_else(|_| vec![0; 24]);
    Some(json!({
        "local_date": local_date,
        "total_screen_time_ms": row.0,
        "active_time_ms": row.1,
        "passive_time_ms": row.2,
        "idle_time_ms": row.3,
        "locked_time_ms": row.4,
        "sleep_time_ms": row.5,
        "monitor_off_time_ms": 0,
        "session_count": row.6,
        "productivity_score": row.7,
        "deep_work_sessions": row.8,
        "deep_work_time_ms": row.9,
        "app_switches": row.10,
        "distraction_index": row.11,
        "network_sent_bytes": row.12,
        "network_recv_bytes": row.13,
        "hourly_breakdown_ms": hourly,
    }))
}
