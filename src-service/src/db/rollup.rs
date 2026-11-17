//! Daily rollup: collapse a completed day's raw rows into one `daily_summaries`
//! aggregate, then optionally delete the raw rows. Preserves every meaningful
//! number (totals, hourly shape, top apps/sites) while shrinking storage hugely.

use chrono::{DateTime, Duration, Local, Timelike, Utc};
use rusqlite::Connection;

/// Read an integer "one" scalar.
fn one(conn: &Connection, sql: &str, date: &str) -> i64 {
    conn.query_row(sql, [date], |r| r.get(0)).unwrap_or(0)
}

/// Screen time bucketed by LOCAL hour.
fn hourly_breakdown(conn: &Connection, date: &str) -> Vec<i64> {
    let mut hourly = vec![0i64; 24];
    let Ok(mut stmt) = conn.prepare(
        "SELECT start_time, end_time FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL",
    ) else {
        return hourly;
    };
    let Ok(rows) = stmt.query_map([date], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }) else {
        return hourly;
    };
    for row in rows.flatten() {
        let (s, e) = row;
        let (Ok(s), Ok(e)) = (
            DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)),
            DateTime::parse_from_rfc3339(&e).map(|d| d.with_timezone(&Utc)),
        ) else {
            continue;
        };
        let mut cur = s.with_timezone(&Local);
        let end = e.with_timezone(&Local);
        if end < cur {
            continue;
        }
        while cur < end {
            let hour = cur.hour() as usize;
            if hour >= 24 {
                break;
            }
            let next_hour = if cur.minute() == 0 && cur.second() == 0 {
                cur + Duration::hours(1)
            } else {
                (cur + Duration::minutes(60 - i64::from(cur.minute())))
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
            };
            let seg_end = if next_hour < end { next_hour } else { end };
            let ms = (seg_end - cur).num_milliseconds();
            if ms > 0 {
                hourly[hour] += ms;
            }
            cur = seg_end;
        }
    }
    hourly
}

/// Per-hour network totals (sent/recv) as JSON: `[{hour, sent, recv}, ...]`.
fn network_hourly_breakdown(conn: &Connection, date: &str) -> String {
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(24);
    let mut per_hour = [((0i64), (0i64)); 24];
    if let Ok(mut stmt) = conn.prepare(
        "SELECT CAST(strftime('%H', datetime(recorded_at, 'localtime')) AS INTEGER),
                      SUM(bytes_sent), SUM(bytes_received)
           FROM network_snapshots WHERE local_date = ?1
          GROUP BY strftime('%H', datetime(recorded_at, 'localtime'))",
    ) {
        if let Ok(rows) = stmt.query_map([date], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for r in rows.flatten() {
                if (r.0 as usize) < 24 {
                    per_hour[r.0 as usize] = (r.1, r.2);
                }
            }
        }
    }
    for h in 0..24 {
        out.push(serde_json::json!({ "hour": h, "sent": per_hour[h].0, "recv": per_hour[h].1 }));
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

/// Top apps JSON for the day.
fn top_apps_json(conn: &Connection, date: &str) -> String {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT process_name, category, COALESCE(SUM(duration_ms),0), COUNT(*)
           FROM app_sessions WHERE local_date = ?1 AND end_time IS NOT NULL
          GROUP BY process_name ORDER BY 3 DESC LIMIT 12",
    ) {
        if let Ok(rows) = stmt.query_map([date], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        }) {
            for r in rows.flatten() {
                out.push(serde_json::json!({
                    "process_name": r.0, "category": r.1, "total_ms": r.2, "sessions": r.3
                }));
            }
        }
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

/// Top sites JSON for the day.
fn top_sites_json(conn: &Connection, date: &str) -> String {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT b.normalized_domain, COUNT(*), COALESCE(SUM(s.duration_ms),0)
           FROM browser_visits b LEFT JOIN app_sessions s ON s.id = b.session_id
          WHERE b.local_date = ?1
          GROUP BY b.normalized_domain ORDER BY 3 DESC LIMIT 12",
    ) {
        if let Ok(rows) = stmt.query_map([date], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for r in rows.flatten() {
                out.push(serde_json::json!({ "domain": r.0, "visits": r.1, "total_ms": r.2 }));
            }
        }
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

/// Build and upsert the `daily_summaries` aggregate for `local_date`.
pub fn rollup_day(conn: &Connection, local_date: &str) -> rusqlite::Result<bool> {
    let total_screen_ms = one(conn, "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions WHERE local_date=?1 AND end_time IS NOT NULL", local_date);
    let total_active_ms = one(conn, "SELECT COALESCE(SUM(CASE WHEN is_browser=1 THEN duration_ms ELSE duration_ms*active_ratio END),0) FROM app_sessions WHERE local_date=?1 AND end_time IS NOT NULL", local_date);
    let session_count = one(conn, "SELECT COUNT(*) FROM app_sessions WHERE local_date=?1", local_date);
    let idle_ms = one(conn, "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods WHERE local_date=?1 AND period_type='idle'", local_date);
    let locked_ms = one(conn, "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods WHERE local_date=?1 AND period_type='locked'", local_date);
    let sleep_ms = one(conn, "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods WHERE local_date=?1 AND period_type='sleep'", local_date);
    let net_sent = one(conn, "SELECT COALESCE(SUM(bytes_sent),0) FROM network_snapshots WHERE local_date=?1", local_date);
    let net_recv = one(conn, "SELECT COALESCE(SUM(bytes_received),0) FROM network_snapshots WHERE local_date=?1", local_date);

    let productive_app_ms = one(conn, "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions WHERE local_date=?1 AND end_time IS NOT NULL AND is_browser=0 AND category IN ('development','productivity','communication','education')", local_date);
    let productive_browser_ms = super::domain_categories::productive_browser_ms(conn, local_date);
    let productive_ms = productive_app_ms + productive_browser_ms;
    let productivity_score = if total_screen_ms > 0 {
        (productive_ms as f64 / total_screen_ms as f64) * 100.0
    } else {
        0.0
    };

    let (deep_count, deep_ms) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(duration_ms),0) FROM app_sessions WHERE local_date=?1 AND end_time IS NOT NULL AND category IN ('development','productivity') AND duration_ms>=2700000",
            [local_date],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap_or((0, 0));

    let active_hours = total_screen_ms as f64 / 3_600_000.0;
    let distraction_index = if active_hours > 0.1 {
        (session_count as f64 / active_hours).round()
    } else {
        0.0
    };

    let hourly = hourly_breakdown(conn, local_date);
    let net_hourly = network_hourly_breakdown(conn, local_date);
    let top_apps = top_apps_json(conn, local_date);
    let top_sites = top_sites_json(conn, local_date);
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO daily_summaries
            (local_date, total_screen_time_ms, total_active_time_ms, total_passive_time_ms,
             total_idle_time_ms, total_locked_time_ms, total_sleep_time_ms,
             session_count, productivity_score, deep_work_sessions, deep_work_time_ms,
             app_switches, distraction_index, total_bytes_sent, total_bytes_received,
             top_apps_json, top_sites_json, hourly_breakdown_json, hourly_network_json, computed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)
         ON CONFLICT(local_date) DO UPDATE SET
            total_screen_time_ms=excluded.total_screen_time_ms,
            total_active_time_ms=excluded.total_active_time_ms,
            total_passive_time_ms=excluded.total_passive_time_ms,
            total_idle_time_ms=excluded.total_idle_time_ms,
            total_locked_time_ms=excluded.total_locked_time_ms,
            total_sleep_time_ms=excluded.total_sleep_time_ms,
            session_count=excluded.session_count,
            productivity_score=excluded.productivity_score,
            deep_work_sessions=excluded.deep_work_sessions,
            deep_work_time_ms=excluded.deep_work_time_ms,
            app_switches=excluded.app_switches,
            distraction_index=excluded.distraction_index,
            total_bytes_sent=excluded.total_bytes_sent,
            total_bytes_received=excluded.total_bytes_received,
            top_apps_json=excluded.top_apps_json,
            top_sites_json=excluded.top_sites_json,
            hourly_breakdown_json=excluded.hourly_breakdown_json,
            hourly_network_json=excluded.hourly_network_json,
            computed_at=excluded.computed_at",
        rusqlite::params![
            local_date, total_screen_ms, total_active_ms, (total_screen_ms - total_active_ms).max(0),
            idle_ms, locked_ms, sleep_ms, session_count, productivity_score, deep_count, deep_ms,
            session_count, distraction_index, net_sent, net_recv, top_apps, top_sites,
            serde_json::to_string(&hourly).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&net_hourly).unwrap_or_else(|_| "[]".into()),
            now,
        ],
    )?;
    Ok(true)
}

/// Delete raw rows for a completed day after its rollup exists.
pub fn drop_raw_day(conn: &Connection, local_date: &str) -> rusqlite::Result<(i64, i64, i64, i64)> {
    let s = conn.execute("DELETE FROM app_sessions WHERE local_date = ?1", [local_date])? as i64;
    let v = conn.execute("DELETE FROM browser_visits WHERE local_date = ?1", [local_date])? as i64;
    let n = conn.execute("DELETE FROM network_snapshots WHERE local_date = ?1", [local_date])? as i64;
    let i = conn.execute("DELETE FROM idle_periods WHERE local_date = ?1", [local_date])? as i64;
    Ok((s, v, n, i))
}

/// Consolidate past days outside the raw-keep window.
pub fn consolidate_past_days(conn: &Connection, keep_raw_days: i64) -> rusqlite::Result<(i64, i64)> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let cutoff = (Local::now() - Duration::days(keep_raw_days)).format("%Y-%m-%d").to_string();

    let mut dates: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT DISTINCT local_date FROM app_sessions WHERE local_date < ?1 ORDER BY local_date")?;
        for r in stmt.query_map([&today], |r| r.get::<_, String>(0))?.flatten() {
            if r.as_str() < cutoff.as_str() && !dates.contains(&r) {
                dates.push(r);
            }
        }
    }
    {
        let mut stmt = conn.prepare("SELECT DISTINCT local_date FROM browser_visits WHERE local_date < ?1")?;
        for r in stmt.query_map([&today], |r| r.get::<_, String>(0))?.flatten() {
            if r.as_str() < cutoff.as_str() && !dates.contains(&r) {
                dates.push(r);
            }
        }
    }

    let mut days_rolled = 0i64;
    let mut rows_removed = 0i64;
    for d in dates {
        rollup_day(conn, &d)?;
        let removed = drop_raw_day(conn, &d)?;
        days_rolled += 1;
        rows_removed += removed.0 + removed.1 + removed.2 + removed.3;
    }
    Ok((days_rolled, rows_removed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::summary::daily;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::apply(&c).unwrap();
        c
    }

    #[test]
    fn rollup_preserves_totals() {
        let conn = mem();
        // Seed two raw sessions for an old day.
        let make = |uuid: &str| crate::db::session::ActiveSession {
            uuid: uuid.into(),
            process_name: "code.exe".into(),
            executable_path: None,
            window_title: "x".into(),
            category: "development".into(),
            project_name: None,
            monitor_index: 0,
            is_browser: false,
            start_time: chrono::Utc::now() - chrono::Duration::days(30),
            local_date: "2026-10-01".into(),
            hwnd: 0,
            db_id: 0,
            active_ticks: 10,
            total_ticks: 10,
            last_active_start: None,
            active_duration_ms: 0,
            last_visit_domain: None,
            last_foreground: None,
        };
        crate::db::session::insert(&conn, &make("r1")).unwrap();
        crate::db::session::insert(&conn, &make("r2")).unwrap();
        crate::db::session::finalize(&conn, "r1", &chrono::Utc::now(), "completed", 0.5).unwrap();
        crate::db::session::finalize(&conn, "r2", &chrono::Utc::now(), "completed", 0.5).unwrap();

        rollup_day(&conn, "2026-10-01").unwrap();
        let summary = daily(&conn, "2026-10-01");
        assert!(summary["total_screen_time_ms"].as_i64().unwrap_or(0) > 0);

        let removed = drop_raw_day(&conn, "2026-10-01").unwrap();
        assert!(removed.0 >= 2);
    }
}
