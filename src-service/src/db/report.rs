//! Report generation (CSV + PDF).

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use rusqlite::Connection;

use super::pdf::render_pdf;

/// Convert a stored UTC RFC3339 string to local `YYYY-MM-DD HH:MM:SS`.
fn to_local(iso: &str) -> String {
    DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Daily,
    Weekly,
    Monthly,
}

impl Granularity {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "daily" | "day" | "d" => Some(Granularity::Daily),
            "weekly" | "week" | "w" => Some(Granularity::Weekly),
            "monthly" | "month" | "m" => Some(Granularity::Monthly),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Granularity::Daily => "daily",
            Granularity::Weekly => "weekly",
            Granularity::Monthly => "monthly",
        }
    }
}

pub struct ReportRow {
    pub label: String,
    pub sessions: i64,
    pub total_ms: i64,
    pub active_ms: i64,
    pub passive_ms: i64,
}

pub struct Report {
    pub title: String,
    pub rows: Vec<ReportRow>,
    pub top_apps: Vec<(String, String, i64)>, // name, category, ms
    pub top_sites: Vec<(String, i64)>, // domain, ms
}

/// Resolve the inclusive date range for a granularity + anchor (local dates).
fn resolve_range(gran: Granularity, anchor: &str) -> Option<(NaiveDate, NaiveDate)> {
    let date = NaiveDate::parse_from_str(anchor, "%Y-%m-%d").ok()?;
    match gran {
        Granularity::Daily => Some((date, date)),
        Granularity::Weekly => {
            // Monday start
            let dow = date.weekday().number_from_monday(); // 1..7
            let monday = date - Duration::days((dow - 1) as i64);
            Some((monday, monday + Duration::days(6)))
        }
        Granularity::Monthly => {
            let y = date.year();
            let m = date.month();
            let first = NaiveDate::from_ymd_opt(y, m, 1)?;
            let last = if m == 12 {
                NaiveDate::from_ymd_opt(y + 1, 1, 1)?
            } else {
                NaiveDate::from_ymd_opt(y, m + 1, 1)?
            } - Duration::days(1);
            Some((first, last))
        }
    }
}

fn fmt_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Build a report (rows + top apps/sites) for a granularity over an anchor date.
pub fn build_report(conn: &Connection, gran: Granularity, anchor: &str) -> Option<Report> {
    let (start, end) = resolve_range(gran, anchor)?;
    let start_s = fmt_date(start);
    let end_s = fmt_date(end);

    // Per-day rows over the range.
    let mut rows = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT local_date, COUNT(*), COALESCE(SUM(duration_ms),0),
                 COALESCE(SUM(CASE WHEN is_browser=1 THEN duration_ms ELSE duration_ms*active_ratio END),0)
            FROM app_sessions
           WHERE local_date BETWEEN ?1 AND ?2 AND end_time IS NOT NULL
           GROUP BY local_date ORDER BY local_date",
    ) {
        if let Ok(iter) = stmt.query_map([start_s.as_str(), end_s.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, f64>(3)?,
            ))
        }) {
            for row in iter.flatten() {
                let total = row.2;
                let active = row.3.round() as i64;
                rows.push(ReportRow {
                    label: row.0,
                    sessions: row.1,
                    total_ms: total,
                    active_ms: active,
                    passive_ms: (total - active).max(0),
                });
            }
        }
    }

    // Top apps in range.
    let mut top_apps = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT process_name, category, COALESCE(SUM(duration_ms),0)
           FROM app_sessions
          WHERE local_date BETWEEN ?1 AND ?2 AND end_time IS NOT NULL
          GROUP BY process_name ORDER BY 3 DESC LIMIT 8",
    ) {
        if let Ok(iter) = stmt.query_map([start_s.as_str(), end_s.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
        }) {
            for row in iter.flatten() {
                top_apps.push(row);
            }
        }
    }

    // Top sites in range.
    let mut top_sites = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT b.normalized_domain, COALESCE(SUM(s.duration_ms),0)
           FROM browser_visits b
           LEFT JOIN app_sessions s ON s.id = b.session_id
          WHERE b.local_date BETWEEN ?1 AND ?2
          GROUP BY b.normalized_domain ORDER BY 2 DESC LIMIT 8",
    ) {
        if let Ok(iter) = stmt.query_map([start_s.as_str(), end_s.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        }) {
            for row in iter.flatten() {
                top_sites.push(row);
            }
        }
    }

    let title = match gran {
        Granularity::Daily => format!("Vigil daily report — {start_s}"),
        Granularity::Weekly => format!("Vigil weekly report — {start_s} → {end_s}"),
        Granularity::Monthly => format!("Vigil monthly report — {start_s} → {end_s}"),
    };

    Some(Report { title, rows, top_apps, top_sites })
}

/// Render the report as CSV text.
pub fn report_csv(report: &Report) -> String {
    let mut out = format!("{},sessions,total_ms,active_ms,passive_ms\n", report.title);
    for r in &report.rows {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            r.label, r.sessions, r.total_ms, r.active_ms, r.passive_ms
        ));
    }
    out.push_str("\nTop applications\nname,category,total_ms\n");
    for (n, c, ms) in &report.top_apps {
        out.push_str(&format!("{n},{c},{ms}\n"));
    }
    out.push_str("\nTop sites\ndomain,total_ms\n");
    for (d, ms) in &report.top_sites {
        out.push_str(&format!("{d},{ms}\n"));
    }
    out
}

/// Render the report as Markdown (Obsidian-friendly daily note).
pub fn report_md(report: &Report) -> String {
    let generated = Utc::now().with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
    let mut out = format!("# {}\n\n", report.title);
    out.push_str(&format!("_Generated {generated} · local time_\n\n"));

    out.push_str("| Date | Sessions | Total | Active | Passive |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for r in &report.rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            r.label, r.sessions, ms_human(r.total_ms), ms_human(r.active_ms), ms_human(r.passive_ms)
        ));
    }

    if !report.top_apps.is_empty() {
        out.push_str("\n## Top applications\n\n");
        for (n, c, ms) in &report.top_apps {
            out.push_str(&format!("- **{n}** ({c}) — {}\n", ms_human(*ms)));
        }
    }
    if !report.top_sites.is_empty() {
        out.push_str("\n## Top sites\n\n");
        for (d, ms) in &report.top_sites {
            out.push_str(&format!("- {d} — {}\n", ms_human(*ms)));
        }
    }
    out
}

/// Render the report as a PDF (vector bytes).
pub fn report_pdf(report: &Report) -> Vec<u8> {
    let generated = Utc::now().with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
    let subtitle = format!("Generated {generated} · local time");
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("{:<12} {:>8} {:>12} {:>12} {:>12}", "Date", "Sess", "Total", "Active", "Passive"));
    for r in &report.rows {
        lines.push(format!(
            "{:<12} {:>8} {:>12} {:>12} {:>12}",
            r.label, r.sessions, ms_human(r.total_ms), ms_human(r.active_ms), ms_human(r.passive_ms)
        ));
    }

    if !report.top_apps.is_empty() {
        lines.push(String::new());
        lines.push("Top applications".to_string());
        for (n, c, ms) in &report.top_apps {
            lines.push(format!("  {n} ({c}) — {}", ms_human(*ms)));
        }
    }
    if !report.top_sites.is_empty() {
        lines.push(String::new());
        lines.push("Top sites".to_string());
        for (d, ms) in &report.top_sites {
            lines.push(format!("  {d} — {}", ms_human(*ms)));
        }
    }

    render_pdf(&report.title, &subtitle, &lines)
}

fn ms_human(ms: i64) -> String {
    let s = ms / 1000;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    if h > 0 {
        format!("{h}h{m}m")
    } else {
        format!("{m}m")
    }
}

/// Export app sessions for a local date as CSV text (contains UTC and local times).
pub fn sessions_csv(conn: &Connection, local_date: &str) -> rusqlite::Result<String> {
    let mut stmt = conn.prepare(
        "SELECT session_uuid, process_name, category, is_browser, start_time, end_time,
                duration_ms, status, active_ratio
           FROM app_sessions
          WHERE local_date = ?1
          ORDER BY start_time",
    )?;
    let mut out = String::from(
        "session_uuid,process_name,category,is_browser,start_time_utc,start_time_local,end_time_utc,end_time_local,duration_ms,status,active_ratio\n",
    );
    let rows = stmt.query_map([local_date], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, f64>(8)?,
        ))
    })?;
    for row in rows {
        let (uuid, app, cat, is_browser, start, end, dur, status, ratio) = row?;
        let end_s = end.as_deref().unwrap_or("");
        let start_local = to_local(&start);
        let end_local = end.as_deref().map(to_local).unwrap_or_default();
        out.push_str(&format!(
            "{uuid},{app},{cat},{is_browser},{start},{start_local},{end_s},{end_local},{duration},{status},{ratio}\n",
            duration = dur.map_or_else(String::new, |d| d.to_string()),
        ));
    }
    Ok(out)
}
