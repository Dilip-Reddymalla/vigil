//! Period-over-period comparison ("Improvements" page).
//!
//! Aggregates `daily_summaries` over two adjacent periods (Mon–Sun for week,
//! calendar months for month) and produces deltas + an overall verdict.

use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::Connection;
use serde_json::{json, Value};

#[derive(Default, Debug, Clone)]
pub struct PeriodMetrics {
    pub screen_ms: i64,
    pub focus: Option<f64>,
    pub deep_ms: i64,
    pub deep_sessions: i64,
    pub distraction: Option<f64>,
    pub active_ratio: Option<f64>,
    pub days: i64,
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Inclusive Monday→Sunday range containing `anchor`.
pub fn week_range(anchor: &str) -> Option<(String, String)> {
    let d = NaiveDate::parse_from_str(anchor, "%Y-%m-%d").ok()?;
    let dow = d.weekday().number_from_monday(); // 1..7
    let monday = d - Duration::days((dow - 1) as i64);
    Some((fmt(monday), fmt(monday + Duration::days(6))))
}

/// Inclusive calendar-month range containing `anchor`.
pub fn month_range(anchor: &str) -> Option<(String, String)> {
    let d = NaiveDate::parse_from_str(anchor, "%Y-%m-%d").ok()?;
    let first = NaiveDate::from_ymd_opt(d.year(), d.month(), 1)?;
    let last = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1)? - Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1)? - Duration::days(1)
    };
    Some((fmt(first), fmt(last)))
}

/// Number of days in an inclusive date range.
fn span_days(start: &str, end: &str) -> i64 {
    let Ok(s) = NaiveDate::parse_from_str(start, "%Y-%m-%d") else { return 0 };
    let Ok(e) = NaiveDate::parse_from_str(end, "%Y-%m-%d") else { return 0 };
    (e - s).num_days() + 1
}

/// Current + previous period ranges (same length) for `period`.
pub fn ranges(period: &str, anchor: &str) -> Option<((String, String), (String, String))> {
    let (cs, ce) = match period {
        "month" => month_range(anchor)?,
        _ => week_range(anchor)?,
    };
    let len = span_days(&cs, &ce).max(1);
    let prev_end = NaiveDate::parse_from_str(&cs, "%Y-%m-%d").ok()? - Duration::days(1);
    let prev_start = prev_end - Duration::days(len - 1);
    Some(((cs, ce), (fmt(prev_start), fmt(prev_end))))
}

/// Aggregate the metrics for an inclusive date range.
fn aggregate(conn: &Connection, start: &str, end: &str) -> PeriodMetrics {
    let mut m = PeriodMetrics::default();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT
            COALESCE(SUM(total_screen_time_ms),0),
            AVG(productivity_score),
            COALESCE(SUM(deep_work_time_ms),0),
            COALESCE(SUM(deep_work_sessions),0),
            AVG(distraction_index),
            AVG(CASE WHEN total_screen_time_ms > 0
                     THEN total_active_time_ms * 1.0 / total_screen_time_ms END)
         FROM daily_summaries WHERE local_date BETWEEN ?1 AND ?2",
    ) {
        if let Ok(row) = stmt.query_row([start, end], |r| {
            Ok(PeriodMetrics {
                screen_ms: r.get::<_, i64>(0)?,
                focus: r.get(1)?,
                deep_ms: r.get::<_, i64>(2)?,
                deep_sessions: r.get::<_, i64>(3)?,
                distraction: r.get(4)?,
                active_ratio: r.get(5)?,
                days: 0,
            })
        }) {
            m = row;
        }
    }
    if let Ok(n) = conn.query_row(
        "SELECT COUNT(*) FROM daily_summaries WHERE local_date BETWEEN ?1 AND ?2",
        [start, end],
        |r| r.get::<_, i64>(0),
    ) {
        m.days = n;
    }
    m
}

fn pct(cur: f64, prev: f64) -> Option<f64> {
    if prev != 0.0 {
        Some((cur - prev) / prev * 100.0)
    } else {
        None
    }
}

/// Compare the two periods and build the frontend response.
pub fn compare(conn: &Connection, period: &str) -> Option<Value> {
    let anchor = chrono::Local::now().format("%Y-%m-%d").to_string();
    let ((cs, ce), (ps, pe)) = ranges(period, &anchor)?;

    let cur = aggregate(conn, &cs, &ce);
    let prev = aggregate(conn, &ps, &pe);

    // Direction-aware "better" flags.
    let screen_better = cur.screen_ms < prev.screen_ms;
    let focus_better = prev.focus.map_or(false, |p| cur.focus.map_or(false, |c| c > p));
    let deep_better = cur.deep_ms > prev.deep_ms;
    let sessions_better = cur.deep_sessions > prev.deep_sessions;
    let distraction_better = prev.distraction.map_or(false, |p| cur.distraction.map_or(false, |c| c < p));
    let active_better = prev.active_ratio.map_or(false, |p| cur.active_ratio.map_or(false, |c| c > p));

    let screen_worse = cur.screen_ms > prev.screen_ms;
    let focus_worse = prev.focus.map_or(false, |p| cur.focus.map_or(true, |c| c < p));
    let deep_worse = cur.deep_ms < prev.deep_ms;

    // Verdict anchored on the user-facing rule: screen time down + focus/deep
    // work up ⇒ "good"; screen up with focus/deep down ⇒ "needs attention".
    let needs_attention = screen_worse && (focus_worse || deep_worse);
    let good = !needs_attention && screen_better && (focus_better || deep_better);
    let verdict = if needs_attention {
        "needs_attention"
    } else if good {
        "good"
    } else {
        "mixed"
    };

    let reasons = build_reasons(&cur, &prev, &screen_better, &focus_better, &deep_better, &sessions_better, &distraction_better, &active_better);

    // Day-by-day series (used by the week tab).
    let daily_cur = daily_series(conn, &cs, &ce);
    let daily_prev = daily_series(conn, &ps, &pe);

    let metrics = |m: &PeriodMetrics| json!({
        "screen_ms": m.screen_ms,
        "focus": m.focus,
        "deep_ms": m.deep_ms,
        "deep_sessions": m.deep_sessions,
        "distraction": m.distraction,
        "active_ratio": m.active_ratio,
        "days": m.days,
    });

    Some(json!({
        "period": period,
        "ranges": { "current": { "start": cs, "end": ce }, "previous": { "start": ps, "end": pe } },
        "current": metrics(&cur),
        "previous": metrics(&prev),
        "deltas": {
            "screen_pct": pct(cur.screen_ms as f64, prev.screen_ms as f64),
            "focus_pct": cur.focus.zip(prev.focus).and_then(|(c, p)| pct(c, p)),
            "deep_pct": pct(cur.deep_ms as f64, prev.deep_ms as f64),
            "deep_sessions_pct": pct(cur.deep_sessions as f64, prev.deep_sessions as f64),
            "distraction_pct": cur.distraction.zip(prev.distraction).and_then(|(c, p)| pct(c, p)),
            "active_pct": cur.active_ratio.zip(prev.active_ratio).and_then(|(c, p)| pct(c, p)),
        },
        "verdict": verdict,
        "reasons": reasons,
        "daily": { "current": daily_cur, "previous": daily_prev },
    }))
}

/// Full measured metrics for a user-selected week (the Monday→Sunday containing
/// `anchor`). Unlike `compare`, this reports a single week's absolute values plus
/// its per-day breakdown — used by the "select a week" tab on the Improvements page.
pub fn week_detail(conn: &Connection, anchor: &str) -> Option<Value> {
    let (start, end) = week_range(anchor)?;
    let m = aggregate(conn, &start, &end);
    let daily = daily_series(conn, &start, &end);

    Some(json!({
        "period": "week_detail",
        "ranges": { "current": { "start": start, "end": end } },
        "metrics": {
            "screen_ms": m.screen_ms,
            "focus": m.focus,
            "deep_ms": m.deep_ms,
            "deep_sessions": m.deep_sessions,
            "distraction": m.distraction,
            "active_ratio": m.active_ratio,
            "days": m.days,
        },
        "daily": daily,
    }))
}

fn build_reasons(
    cur: &PeriodMetrics,
    prev: &PeriodMetrics,
    screen_better: &bool,
    focus_better: &bool,
    deep_better: &bool,
    sessions_better: &bool,
    distraction_better: &bool,
    active_better: &bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let mk = |label: &str, delta: Option<f64>, better: bool| -> Option<String> {
        match delta {
            Some(d) => {
                let sign = if d < 0.0 { "↓" } else { "↑" };
                Some(format!(
                    "{label} {sign} {:.0}% {}",
                    d.abs(),
                    if better { "✓" } else { "✗" }
                ))
            }
            None => None,
        }
    };
    if let Some(s) = mk("screen time", pct(cur.screen_ms as f64, prev.screen_ms as f64), *screen_better) {
        out.push(s);
    }
    if let Some(s) = mk("focus score", cur.focus.zip(prev.focus).and_then(|(c, p)| pct(c, p)), *focus_better) {
        out.push(s);
    }
    if let Some(s) = mk("deep work", pct(cur.deep_ms as f64, prev.deep_ms as f64), *deep_better) {
        out.push(s);
    }
    if let Some(s) = mk("deep-work sessions", pct(cur.deep_sessions as f64, prev.deep_sessions as f64), *sessions_better) {
        out.push(s);
    }
    if let Some(s) = mk("distraction", cur.distraction.zip(prev.distraction).and_then(|(c, p)| pct(c, p)), *distraction_better) {
        out.push(s);
    }
    if let Some(s) = mk("active ratio", cur.active_ratio.zip(prev.active_ratio).and_then(|(c, p)| pct(c, p)), *active_better) {
        out.push(s);
    }
    out
}

fn daily_series(conn: &Connection, start: &str, end: &str) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT local_date, total_screen_time_ms, deep_work_time_ms, productivity_score
           FROM daily_summaries WHERE local_date BETWEEN ?1 AND ?2 ORDER BY local_date",
    ) {
        if let Ok(rows) = stmt.query_map([start, end], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<f64>>(3)?,
            ))
        }) {
            for row in rows.flatten() {
                out.push(json!({ "date": row.0, "screen_ms": row.1, "deep_ms": row.2, "focus": row.3 }));
            }
        }
    }
    out
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

    fn seed(conn: &Connection, date: &str, screen_ms: i64, focus: f64, deep_ms: i64) {
        conn.execute(
            "INSERT INTO daily_summaries
                (local_date, total_screen_time_ms, total_active_time_ms, productivity_score,
                 deep_work_time_ms, deep_work_sessions, distraction_index, computed_at)
             VALUES (?1, ?2, ?2, ?3, ?4, 1, 20, '2026-01-01T00:00:00Z')",
            rusqlite::params![date, screen_ms, focus, deep_ms],
        )
        .unwrap();
    }

    #[test]
    fn week_range_is_monday_to_sunday() {
        let (s, e) = week_range("2026-11-18").unwrap(); // Wednesday
        assert_eq!(s, "2026-11-16");
        assert_eq!(e, "2026-11-22");
    }

    #[test]
    fn month_range_bounds() {
        let (s, e) = month_range("2026-11-18").unwrap();
        assert_eq!(s, "2026-11-01");
        assert_eq!(e, "2026-11-30");
    }

    /// Seed all 7 days of the Monday→Sunday week containing `anchor`.
    fn seed_week(conn: &Connection, anchor: &str, screen_ms: i64, focus: f64, deep_ms: i64) {
        let (s, e) = week_range(anchor).unwrap();
        let start_date = NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap();
        for i in 0..7 {
            let d = start_date + Duration::days(i);
            seed(conn, &fmt(d), screen_ms, focus, deep_ms);
        }
    }

    #[test]
    fn verdict_good_when_screen_down_deep_up() {
        let conn = mem();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        // Previous week (the Monday before this week's Monday): high screen, low focus/deep.
        let prev_monday = NaiveDate::parse_from_str(week_range(&today).unwrap().0.as_str(), "%Y-%m-%d").unwrap()
            - Duration::days(1);
        seed_week(&conn, &fmt(prev_monday), 3_600_000, 50.0, 1_800_000);
        // Current week: less screen, more focus/deep.
        seed_week(&conn, &today, 2_400_000, 60.0, 2_800_000);
        let v = compare(&conn, "week").unwrap();
        assert_eq!(v["verdict"], "good");
    }

    #[test]
    fn week_detail_reports_selected_week() {
        let conn = mem();
        // Seed Mon-Sun 2026-11-16..22
        for d in 16..=22 {
            seed(&conn, &format!("2026-11-{d:02}"), 1_000_000, 55.0, 500_000);
        }
        // A different week (2026-11-09..15) that must NOT leak in.
        for d in 9..=11 {
            seed(&conn, &format!("2026-11-{d:02}"), 9_000_000, 10.0, 0);
        }
        let v = week_detail(&conn, "2026-11-20").unwrap(); // Thursday → that week
        let m = &v["metrics"];
        // 7 days × 1,000,000 ms
        assert_eq!(m["screen_ms"], 7_000_000);
        assert_eq!(m["deep_ms"], 3_500_000);
        assert_eq!(m["deep_sessions"], 7);
        assert_eq!(m["days"], 7);
        assert_eq!(v["daily"].as_array().map(|a| a.len()), Some(7));
    }

    #[test]
    fn verdict_needs_attention_when_screen_up_focus_down() {
        let conn = mem();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let prev_monday = NaiveDate::parse_from_str(week_range(&today).unwrap().0.as_str(), "%Y-%m-%d").unwrap()
            - Duration::days(1);
        seed_week(&conn, &fmt(prev_monday), 2_400_000, 60.0, 2_000_000);
        seed_week(&conn, &today, 4_200_000, 40.0, 1_000_000);
        let v = compare(&conn, "week").unwrap();
        assert_eq!(v["verdict"], "needs_attention");
    }
}
