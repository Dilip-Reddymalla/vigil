//! Idle / non-screen period recording (with detection source).

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// Insert an idle/locked/sleep/monitor_off period.
pub fn insert(
    conn: &Connection,
    period_type: &str,
    source: &str,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    local_date: &str,
) -> rusqlite::Result<()> {
    let duration_ms = (*end - *start).num_milliseconds().max(0);
    conn.execute(
        "INSERT INTO idle_periods
            (period_type, detection_source, start_time, end_time, duration_ms, local_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            period_type,
            source,
            start.to_rfc3339(),
            end.to_rfc3339(),
            duration_ms,
            local_date
        ],
    )?;
    Ok(())
}
