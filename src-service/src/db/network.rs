//! Network snapshot persistence (interface-level, counter-safe).

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// Insert one interface snapshot.
#[allow(clippy::too_many_arguments)]
pub fn insert_snapshot(
    conn: &Connection,
    interface_name: &str,
    interface_luid: &str,
    interface_type: &str,
    bytes_sent: u64,
    bytes_received: u64,
    cumulative_sent: u64,
    cumulative_recv: u64,
    recorded_at: &DateTime<Utc>,
    local_date: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO network_snapshots
            (interface_name, interface_luid, interface_type, bytes_sent, bytes_received,
             cumulative_sent, cumulative_recv, recorded_at, local_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            interface_name,
            interface_luid,
            interface_type,
            bytes_sent as i64,
            bytes_received as i64,
            cumulative_sent as i64,
            cumulative_recv as i64,
            recorded_at.to_rfc3339(),
            local_date
        ],
    )?;
    Ok(())
}

/// Total sent/received bytes for a local date.
pub fn daily_totals(conn: &Connection, local_date: &str) -> (i64, i64) {
    let sent: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(bytes_sent),0) FROM network_snapshots WHERE local_date = ?1",
            [local_date],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let recv: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(bytes_received),0) FROM network_snapshots WHERE local_date = ?1",
            [local_date],
            |r| r.get(0),
        )
        .unwrap_or(0);
    (sent, recv)
}
