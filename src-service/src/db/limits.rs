//! Daily app/domain usage limits (Phase 7.1, backend).
//!
//! Limits are stored as a JSON setting (`usage_limits`): an array of
//! `{ "type": "app"|"domain", "name": "...", "minutes": 120 }`. We compute each
//! limit's current usage for a day and its fill %, so the service can nudge at
//! 90% / 100%.

use rusqlite::Connection;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LimitStatus {
    pub kind: String, // "app" | "domain"
    pub name: String,
    pub used_ms: i64,
    pub limit_ms: i64,
    pub pct: i64, // 0..100 (or >100 when exceeded)
}

/// Read the configured limits (empty if none).
pub fn read_limits(conn: &Connection) -> Vec<Value> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = 'usage_limits'", [], |r| r.get(0))
        .ok();
    let Some(raw) = raw else { return vec![] };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else { return vec![] };
    v.as_array().cloned().unwrap_or_default()
}

/// Compute current usage for each limit.
pub fn compute_limits(conn: &Connection, date: &str, limits: &[Value]) -> Vec<LimitStatus> {
    let mut out = Vec::new();
    for l in limits {
        let kind = l.get("type").and_then(|x| x.as_str()).unwrap_or("app");
        let name = l.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let minutes = l.get("minutes").and_then(|x| x.as_i64()).unwrap_or(0);
        if name.is_empty() || minutes <= 0 {
            continue;
        }
        let limit_ms = minutes * 60_000;
        let used_ms = match kind {
            "domain" => domain_used(conn, date, &name),
            _ => app_used(conn, date, &name),
        };
        let pct = if limit_ms > 0 { (used_ms * 100) / limit_ms } else { 0 };
        out.push(LimitStatus { kind: kind.to_string(), name, used_ms, limit_ms, pct });
    }
    out
}

fn app_used(conn: &Connection, date: &str, process: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL AND process_name = ?2",
        [date, process],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn domain_used(conn: &Connection, date: &str, domain: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(SUM(s.duration_ms),0)
           FROM (SELECT DISTINCT session_id FROM browser_visits
                  WHERE local_date = ?1 AND normalized_domain = ?2) b
           LEFT JOIN app_sessions s ON s.id = b.session_id",
        [date, domain],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::session::{ActiveSession, insert, finalize};
    use chrono::Utc;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::apply(&c).unwrap();
        c
    }

    #[test]
    fn computes_app_limit_usage() {
        let conn = mem();
        let s = ActiveSession {
            uuid: "lim1".into(),
            process_name: "steam.exe".into(),
            executable_path: None,
            window_title: String::new(),
            category: "entertainment".into(),
            project_name: None,
            monitor_index: 0,
            is_browser: false,
            start_time: Utc::now() - chrono::Duration::minutes(10),
            local_date: "2026-11-17".into(),
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
        finalize(&conn, "lim1", &Utc::now(), "completed", 1.0).unwrap();

        let limits = serde_json::json!([
            { "type": "app", "name": "steam.exe", "minutes": 20 }
        ]);
        let statuses = compute_limits(&conn, "2026-11-17", limits.as_array().unwrap());
        assert_eq!(statuses.len(), 1);
        // ~10 minutes used of 20 → ~50%
        assert!(statuses[0].pct >= 40 && statuses[0].pct <= 60, "pct={}", statuses[0].pct);
    }
}
