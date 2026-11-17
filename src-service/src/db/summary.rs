//! Read-side daily summary aggregation.
//!
//! Derived data. All queries filter by the precomputed `local_date`. Hour buckets
//! are allocated in LOCAL time across the actual hours each session spans.

use chrono::{DateTime, Duration, Local, NaiveDate, Timelike, Utc};
use rusqlite::Connection;
use serde_json::json;

/// Daily summary for a `local_date`. If the day was rolled up (no raw rows),
/// reads from `daily_summaries` so historical views still work.
pub fn daily(conn: &Connection, local_date: &str) -> serde_json::Value {
    let raw_count: i64 = one(
        conn,
        "SELECT COUNT(*) FROM app_sessions WHERE local_date = ?1",
        local_date,
    );
    if raw_count == 0 {
        if let Ok(row) = conn.query_row(
            "SELECT total_screen_time_ms, total_active_time_ms, total_passive_time_ms,
                    total_idle_time_ms, total_locked_time_ms, total_sleep_time_ms,
                    session_count, productivity_score, deep_work_sessions, deep_work_time_ms,
                    app_switches, distraction_index, total_bytes_sent, total_bytes_received,
                    hourly_breakdown_json
               FROM daily_summaries WHERE local_date = ?1",
            [local_date],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, f64>(7)?,
                    r.get::<_, i64>(8)?,
                    r.get::<_, i64>(9)?,
                    r.get::<_, i64>(10)?,
                    r.get::<_, f64>(11)?,
                    r.get::<_, i64>(12)?,
                    r.get::<_, i64>(13)?,
                    r.get::<_, String>(14)?,
                ))
            },
        ) {
            let hourly: Vec<i64> = serde_json::from_str(&row.14).unwrap_or_else(|_| vec![0; 24]);
            return json!({
                "local_date": local_date,
                "total_screen_time_ms": row.0,
                "active_time_ms": row.1,
                "passive_time_ms": row.2,
                "idle_time_ms": row.3,
                "locked_time_ms": row.4,
                "sleep_time_ms": row.5,
                "monitor_off_time_ms": 0,
                "session_count": row.6,
                "hourly_breakdown_ms": hourly,
                "productivity_score": row.7,
                "deep_work_sessions": row.8,
                "deep_work_time_ms": row.9,
                "app_switches": row.10,
                "distraction_index": row.11,
                "network_sent_bytes": row.12,
                "network_recv_bytes": row.13,
                "goal_met": false,
                "goal_streak_days": 0,
                "goal_ms": read_int_setting(conn, "goal_hours", 8) * 3_600_000,
            });
        }
    }

    let total_screen_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL",
        local_date,
    );
    let session_count: i64 = one(
        conn,
        "SELECT COUNT(*) FROM app_sessions WHERE local_date = ?1",
        local_date,
    );
    let idle_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods
          WHERE local_date = ?1 AND period_type = 'idle'",
        local_date,
    );
    let locked_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods
          WHERE local_date = ?1 AND period_type = 'locked'",
        local_date,
    );
    let sleep_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods
          WHERE local_date = ?1 AND period_type = 'sleep'",
        local_date,
    );
    let monitor_off_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM idle_periods
          WHERE local_date = ?1 AND period_type = 'monitor_off'",
        local_date,
    );
    let (network_sent_bytes, network_recv_bytes) = super::network::daily_totals(conn, local_date);

    // Focus score: share of screen time in productive categories. Non-browser
    // sessions use their app category; browser time is attributed by domain
    // category (Phase 5) so entertainment/social sites don't inflate the score.
    let productive_app_ms: i64 = one(
        conn,
        "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL
            AND is_browser = 0
            AND category IN ('development','productivity','communication','education')",
        local_date,
    );
    let productive_browser_ms = super::domain_categories::productive_browser_ms(conn, local_date);
    let productive_ms = productive_app_ms + productive_browser_ms;
    let productivity_score = if total_screen_ms > 0 {
        ((productive_ms as f64 / total_screen_ms as f64) * 100.0).round()
    } else {
        0.0
    };

    // App switches ≈ number of completed sessions; distraction index = switches per active hour.
    let app_switches: i64 = one(
        conn,
        "SELECT COUNT(*) FROM app_sessions WHERE local_date = ?1 AND end_time IS NOT NULL",
        local_date,
    );
    let active_hours = total_screen_ms as f64 / 3_600_000.0;
    let distraction_index = if active_hours > 0.1 {
        (app_switches as f64 / active_hours * 10.0).round() / 10.0
    } else {
        0.0
    };

    // Deep work: long, uninterrupted sessions in productive categories.
    let mut deep_work_sessions: i64 = 0;
    let mut deep_work_time_ms: i64 = 0;
    if let Ok(mut stmt) = conn.prepare(
        "SELECT COUNT(*), COALESCE(SUM(duration_ms),0) FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL
            AND category IN ('development','productivity') AND duration_ms >= 2700000",
    ) {
        if let Ok((n, ms)) = stmt.query_row([local_date], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))) {
            deep_work_sessions = n;
            deep_work_time_ms = ms;
        }
    }

    // Goal (hours) from settings, default 8h.
    let goal_ms = read_int_setting(conn, "goal_hours", 8) * 3_600_000;
    let goal_met = total_screen_ms >= goal_ms;
    let goal_streak_days = compute_goal_streak(conn, local_date, goal_ms);

    // Split screen time into active vs passive.
    // Browser sessions count fully as ACTIVE (watching/reading = engaged), even
    // with no recent input. Non-browser sessions use their input-based active_ratio.
    let active_ms_f: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN is_browser = 1 THEN duration_ms
                                      ELSE duration_ms * active_ratio END),0)
               FROM app_sessions
              WHERE local_date = ?1 AND end_time IS NOT NULL",
            [local_date],
            |r| r.get::<_, f64>(0),
        )
        .unwrap_or(0.0);
    let active_ms = active_ms_f.round() as i64;
    let passive_ms = (total_screen_ms - active_ms).max(0);

    // Screen-time per LOCAL hour (00..24), allocating each session's duration
    // across the hours it actually spans.
    let mut hourly = vec![0i64; 24];
    if let Ok(mut stmt) = conn.prepare(
        "SELECT start_time, end_time FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL",
    ) {
        if let Ok(rows) = stmt.query_map([local_date], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
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
                    let next_hour = cur + Duration::minutes(60 - i64::from(cur.minute()));
                    let next_hour = if cur.minute() == 0 && cur.second() == 0 {
                        cur + Duration::hours(1)
                    } else {
                        next_hour
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
        }
    }

    json!({
        "local_date": local_date,
        "total_screen_time_ms": total_screen_ms,
        "active_time_ms": active_ms,
        "passive_time_ms": passive_ms,
        "idle_time_ms": idle_ms,
        "locked_time_ms": locked_ms,
        "sleep_time_ms": sleep_ms,
        "monitor_off_time_ms": monitor_off_ms,
        "session_count": session_count,
        "network_sent_bytes": network_sent_bytes,
        "network_recv_bytes": network_recv_bytes,
        "hourly_breakdown_ms": hourly,
        "productivity_score": productivity_score,
        "distraction_index": distraction_index,
        "deep_work_sessions": deep_work_sessions,
        "deep_work_time_ms": deep_work_time_ms,
        "goal_met": goal_met,
        "goal_streak_days": goal_streak_days,
        "goal_ms": goal_ms,
        "focus_streak_days": compute_focus_streak(conn, local_date),
    })
}

fn one(conn: &Connection, sql: &str, local_date: &str) -> i64 {
    conn.query_row(sql, [local_date], |r| r.get(0)).unwrap_or(0)
}

/// Read an integer setting from the `settings` table, falling back to `default`.
fn read_int_setting(conn: &Connection, key: &str, default: i64) -> i64 {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse::<i64>().ok())
    .unwrap_or(default)
}

/// Count consecutive days (ending at `anchor`) that had at least one deep-work
/// session (≥ 45 min uninterrupted dev/productivity) — a "flow streak".
fn compute_focus_streak(conn: &Connection, anchor: &str) -> i64 {
    let Ok(anchor_date) = NaiveDate::parse_from_str(anchor, "%Y-%m-%d") else {
        return 0;
    };
    let mut had_deep: std::collections::HashSet<NaiveDate> = Default::default();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT local_date FROM app_sessions
          WHERE end_time IS NOT NULL AND duration_ms >= 2700000
            AND category IN ('development','productivity') AND local_date <= ?1",
    ) {
        if let Ok(rows) = stmt.query_map([anchor], |r| r.get::<_, String>(0)) {
            for row in rows.flatten() {
                if let Ok(d) = NaiveDate::parse_from_str(&row, "%Y-%m-%d") {
                    had_deep.insert(d);
                }
            }
        }
    }
    let mut streak = 0i64;
    let mut day = anchor_date;
    loop {
        if had_deep.contains(&day) {
            streak += 1;
            day -= Duration::days(1);
        } else {
            break;
        }
    }
    streak
}

/// Count consecutive days (ending at `anchor`) where daily screen time ≥ goal.
fn compute_goal_streak(conn: &Connection, anchor: &str, goal_ms: i64) -> i64 {
    let Ok(anchor_date) = NaiveDate::parse_from_str(anchor, "%Y-%m-%d") else {
        return 0;
    };
    let mut day_totals: std::collections::HashMap<NaiveDate, i64> = Default::default();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT local_date, COALESCE(SUM(duration_ms),0) FROM app_sessions
          WHERE end_time IS NOT NULL AND local_date <= ?1
          GROUP BY local_date ORDER BY local_date DESC LIMIT 60",
    ) {
        if let Ok(rows) = stmt.query_map([anchor], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        }) {
            for row in rows.flatten() {
                if let Ok(d) = NaiveDate::parse_from_str(&row.0, "%Y-%m-%d") {
                    day_totals.insert(d, row.1);
                }
            }
        }
    }

    let mut streak = 0i64;
    let mut day = anchor_date;
    loop {
        match day_totals.get(&day) {
            Some(&total) if total >= goal_ms => {
                streak += 1;
                day -= Duration::days(1);
            }
            _ => break,
        }
    }
    streak
}
