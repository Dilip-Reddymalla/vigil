//! App session persistence + startup recovery.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// An open application session (screen time in a foreground app).
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub uuid: String,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub window_title: String,
    pub category: String,
    pub project_name: Option<String>,
    pub monitor_index: u32,
    pub is_browser: bool,
    pub start_time: DateTime<Utc>,
    pub local_date: String,
    /// Raw window handle (isize) — differentiates split-screen windows.
    pub hwnd: isize,
    /// DB row id (filled after insert; used to link visits without a subquery, H2).
    pub db_id: i64,
    /// Input-poller ticks while this session was open (kept for backward compat).
    pub active_ticks: u32,
    pub total_ticks: u32,
    /// Wall-clock active tracking (C4): when the session became active, and total
    /// milliseconds of active time accumulated so far.
    pub last_active_start: Option<DateTime<Utc>>,
    pub active_duration_ms: i64,
    /// Last domain recorded for this session (for detecting tab/site switches).
    pub last_visit_domain: Option<String>,
    /// `(process_name, title, unix_ms)` last seen — used to debounce the
    /// transient shell flicker that fragments sessions.
    pub last_foreground: Option<(String, String, i64)>,
}

/// Record an app launch.
pub fn insert_launch(
    conn: &Connection,
    process_name: &str,
    executable_path: Option<&str>,
    launched_at: &DateTime<Utc>,
    local_date: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO app_launches (process_name, executable_path, launched_at, local_date)
         VALUES (?1, ?2, ?3, ?4)",
        params![process_name, executable_path, launched_at.to_rfc3339(), local_date],
    )?;
    Ok(())
}

/// Insert a new open session. Returns its row id.
pub fn insert(conn: &Connection, s: &ActiveSession) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO app_sessions
            (session_uuid, process_name, executable_path, window_title, category,
             project_name, monitor_index, is_browser, start_time, local_date, status, active_ratio)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', 1.0)",
        params![
            s.uuid,
            s.process_name,
            s.executable_path,
            s.window_title,
            s.category,
            s.project_name,
            s.monitor_index,
            s.is_browser as i32,
            s.start_time.to_rfc3339(),
            s.local_date
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Finalize a session by end time, outcome status, and active ratio.
pub fn finalize(
    conn: &Connection,
    uuid: &str,
    end: &DateTime<Utc>,
    status: &str,
    active_ratio: f64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE app_sessions
            SET end_time = ?1,
                duration_ms = CAST((julianday(?1) - julianday(start_time)) * 86400000 AS INTEGER),
                status = ?2,
                active_ratio = ?4
          WHERE session_uuid = ?3",
        params![end.to_rfc3339(), status, uuid, active_ratio],
    )?;
    Ok(())
}

/// Reconcile orphaned open sessions on startup using the last heartbeat timestamp.
///
/// Returns `(recovered_count, reconciled_seconds)`.
pub fn recover_open(conn: &Connection, last_alive_at: Option<&str>) -> rusqlite::Result<(i64, i64)> {
    let mut stmt = conn.prepare(
        "SELECT session_uuid, start_time FROM app_sessions WHERE status = 'active'",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut recovered: i64 = 0;
    let mut seconds: i64 = 0;

    for row in rows {
        let (uuid, start) = row?;
        let start_dt = DateTime::parse_from_rfc3339(&start)
            .ok()
            .map(|d| d.with_timezone(&Utc));
        let end = last_alive_at.map(str::to_string).unwrap_or_else(|| start.clone());
        let end_dt = DateTime::parse_from_rfc3339(&end)
            .ok()
            .map(|d| d.with_timezone(&Utc));

        let duration_ms = match (start_dt, end_dt) {
            (Some(a), Some(b)) => {
                let ms = (b - a).num_milliseconds().max(0);
                seconds += ms / 1000;
                ms
            }
            _ => 0,
        };

        conn.execute(
            "UPDATE app_sessions SET end_time = ?1, duration_ms = ?2, status = 'recovered'
              WHERE session_uuid = ?3",
            params![end, duration_ms, uuid],
        )?;
        recovered += 1;
    }

    Ok((recovered, seconds))
}

/// Purge raw events older than `keep_days` (default 90). Returns rows removed.
pub fn purge_old(conn: &Connection, keep_days: i64) -> rusqlite::Result<(i64, i64, i64)> {
    let cutoff = (Utc::now() - chrono::Duration::days(keep_days))
        .format("%Y-%m-%d")
        .to_string();
    let sessions = conn.execute(
        "DELETE FROM app_sessions WHERE local_date < ?1",
        [&cutoff],
    )?;
    let visits = conn.execute(
        "DELETE FROM browser_visits WHERE local_date < ?1",
        [&cutoff],
    )?;
    let net = conn.execute(
        "DELETE FROM network_snapshots WHERE local_date < ?1",
        [&cutoff],
    )?;
    Ok((sessions as i64, visits as i64, net as i64))
}

/// Delete a session row (used to drop micro-sessions that get merged away).
pub fn delete_session(conn: &Connection, uuid: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM app_sessions WHERE session_uuid = ?1", [uuid])?;
    Ok(())
}

/// Merge a session that spans a local-midnight boundary into a single row: the
/// row keeps the original start day but the new `end`/`duration` are applied.
pub fn extend_to_end(conn: &Connection, uuid: &str, end: &DateTime<Utc>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE app_sessions
            SET end_time = ?1,
                duration_ms = CAST((julianday(?1) - julianday(start_time)) * 86400000 AS INTEGER)
          WHERE session_uuid = ?2",
        params![end.to_rfc3339(), uuid],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::apply(&c).unwrap();
        c
    }

    #[test]
    fn insert_finalize_roundtrip() {
        let conn = mem();
        let start = Utc::now() - chrono::Duration::minutes(5);
        let s = ActiveSession {
            uuid: "u1".into(),
            process_name: "code.exe".into(),
            executable_path: None,
            window_title: "Editor".into(),
            category: "development".into(),
            project_name: None,
            monitor_index: 0,
            is_browser: false,
            start_time: start,
            local_date: "2026-11-13".into(),
            hwnd: 0,
            db_id: 0,
            active_ticks: 10,
            total_ticks: 10,
            last_active_start: None,
            active_duration_ms: 0,
            last_visit_domain: None,
            last_foreground: None,
        };
        insert(&conn, &s).unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM app_sessions WHERE session_uuid = 'u1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");

        finalize(&conn, "u1", &Utc::now(), "completed", 0.5).unwrap();
        let (status, dur): (String, Option<i64>) = conn
            .query_row(
                "SELECT status, duration_ms FROM app_sessions WHERE session_uuid = 'u1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "completed");
        assert!(dur.unwrap() > 0);
    }

    #[test]
    fn recover_closes_orphans() {
        let conn = mem();
        let start = Utc::now() - chrono::Duration::minutes(10);
        let s = ActiveSession {
            uuid: "u2".into(),
            process_name: "x.exe".into(),
            executable_path: None,
            window_title: String::new(),
            category: "other".into(),
            project_name: None,
            monitor_index: 0,
            is_browser: false,
            start_time: start,
            local_date: "2026-11-13".into(),
            hwnd: 0,
            db_id: 0,
            active_ticks: 0,
            total_ticks: 0,
            last_active_start: None,
            active_duration_ms: 0,
            last_visit_domain: None,
            last_foreground: None,
        };
        insert(&conn, &s).unwrap();

        let last_alive = (Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
        let (recovered, seconds) = recover_open(&conn, Some(&last_alive)).unwrap();
        assert_eq!(recovered, 1);
        assert!(seconds >= 500, "expected ~9 min reconciled, got {seconds}");

        let status: String = conn
            .query_row(
                "SELECT status FROM app_sessions WHERE session_uuid = 'u2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "recovered");
    }
}
