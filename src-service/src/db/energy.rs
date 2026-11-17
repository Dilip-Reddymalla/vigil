//! Battery / energy impact tracking (Phase 9).
//!
//! Polls of battery charge snapshots lets us estimate a per-hour drain rate and
//! attribute a share of that drain to the apps that were foreground during the
//! interval. Everything is computed from the `battery_snapshots` table plus the
//! existing `app_sessions` durations — no extra polling beyond the 60s cadence.

use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct BatterySnapshot {
    pub local_date: String,
    pub ts: String,
    pub ac_line_status: u8,
    pub battery_life_percent: u8,
}

/// Record one battery sample.
pub fn record_battery(
    conn: &Connection,
    local_date: &str,
    ts: &str,
    ac_line_status: u8,
    battery_life_percent: u8,
    battery_flag: u8,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO battery_snapshots
            (local_date, ts, ac_line_status, battery_life_percent, battery_flag)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![local_date, ts, ac_line_status, battery_life_percent, battery_flag],
    )?;
    Ok(())
}

/// Fetch the day's samples ordered by time (ascending).
fn snapshots_for_day(conn: &Connection, local_date: &str) -> Vec<BatterySnapshot> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT local_date, ts, ac_line_status, battery_life_percent
           FROM battery_snapshots
          WHERE local_date = ?1 ORDER BY ts",
    ) {
        if let Ok(rows) = stmt.query_map([local_date], |r| {
            Ok(BatterySnapshot {
                local_date: r.get(0)?,
                ts: r.get(1)?,
                ac_line_status: r.get(2)?,
                battery_life_percent: r.get(3)?,
            })
        }) {
            for r in rows.flatten() {
                out.push(r);
            }
        }
    }
    out
}

/// Estimate battery drain in percent-per-hour for a day, considering only the
/// stretch of time the machine was on battery (ACLineStatus = 0) with a known
/// charge. Returns `None` when there isn't enough data / it's on AC throughout.
pub fn battery_drain_pct_per_hour(conn: &Connection, local_date: &str) -> Option<f64> {
    let snaps = snapshots_for_day(conn, local_date);
    if snaps.len() < 2 {
        return None;
    }
    // Keep only on-battery samples with a sane percent (0=unknown, so treat
    // percent==0 without a flag as unusable, but 0 is also a valid "empty").
    let on_battery: Vec<&BatterySnapshot> = snaps
        .iter()
        .filter(|s| s.ac_line_status == 0 && s.battery_life_percent != 255)
        .collect();
    if on_battery.len() < 2 {
        return None;
    }
    let first = on_battery.first()?;
    let last = on_battery.last()?;
    let t0 = chrono::DateTime::parse_from_rfc3339(&first.ts).ok()?;
    let t1 = chrono::DateTime::parse_from_rfc3339(&last.ts).ok()?;
    let hours = (t1 - t0).num_seconds() as f64 / 3600.0;
    if hours <= 0.0 {
        return None;
    }
    let drop = first.battery_life_percent as f64 - last.battery_life_percent as f64;
    if drop <= 0.0 {
        return None;
    }
    Some(drop / hours)
}

/// Attribute the day's total battery drain to apps by foreground duration.
/// Returns app -> estimated drain % of the day's total drain (or 0 when on AC).
pub fn energy_by_app(conn: &Connection, local_date: &str) -> Vec<(String, i64, f64)> {
    let total_drain = battery_drain_pct_per_hour(conn, local_date).unwrap_or(0.0);
    // Total active duration of non-browser foreground sessions for the day.
    let total_ms: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(duration_ms),0) FROM app_sessions
              WHERE local_date = ?1 AND end_time IS NOT NULL",
            [local_date],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if total_drain <= 0.0 || total_ms <= 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT process_name, COALESCE(SUM(duration_ms),0)
           FROM app_sessions
          WHERE local_date = ?1 AND end_time IS NOT NULL
          GROUP BY process_name ORDER BY 2 DESC LIMIT 12",
    ) {
        if let Ok(rows) = stmt.query_map([local_date], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        }) {
            for row in rows.flatten() {
                let share = row.1 as f64 / total_ms as f64;
                // Approx drain contribution = share of time × total drain (%). We
                // report it as a fraction of the day's total battery drop.
                out.push((row.0, row.1, share * total_drain));
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

    #[test]
    fn drain_rate_from_two_snapshots() {
        let conn = mem();
        record_battery(&conn, "2026-11-17", "2026-11-17T09:00:00Z", 0, 100, 0).unwrap();
        record_battery(&conn, "2026-11-17", "2026-11-17T10:00:00Z", 0, 90, 0).unwrap();
        let rate = battery_drain_pct_per_hour(&conn, "2026-11-17").unwrap();
        assert!((rate - 10.0).abs() < 0.01, "rate={rate}");
    }

    #[test]
    fn ac_only_yields_none() {
        let conn = mem();
        record_battery(&conn, "2026-11-17", "2026-11-17T09:00:00Z", 1, 100, 1).unwrap();
        record_battery(&conn, "2026-11-17", "2026-11-17T10:00:00Z", 1, 100, 1).unwrap();
        assert_eq!(battery_drain_pct_per_hour(&conn, "2026-11-17"), None);
    }

    #[test]
    fn insufficient_data_yields_none() {
        let conn = mem();
        record_battery(&conn, "2026-11-17", "2026-11-17T09:00:00Z", 0, 100, 0).unwrap();
        assert_eq!(battery_drain_pct_per_hour(&conn, "2026-11-17"), None);
    }
}
