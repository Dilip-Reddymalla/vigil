//! Read-side queries backing the dashboard views (applications/websites/network/timeline).

use rusqlite::Connection;

/// Read raw top apps for a local date. Empty if the day was rolled up.
fn raw_top_applications(conn: &Connection, local_date: &str) -> Vec<(String, String, i64, i64)> {
    let mut stmt = match conn.prepare(
        "SELECT process_name, category, COALESCE(SUM(duration_ms),0), COUNT(*)
           FROM app_sessions WHERE local_date = ?1 AND end_time IS NOT NULL
          GROUP BY process_name ORDER BY 3 DESC LIMIT 12",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([local_date], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    });
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Read raw top domains for a local date.
fn raw_top_websites(conn: &Connection, local_date: &str) -> Vec<(String, i64, i64)> {
    let mut stmt = match conn.prepare(
        "SELECT b.normalized_domain, COUNT(*), COALESCE(SUM(s.duration_ms),0)
           FROM browser_visits b
           LEFT JOIN app_sessions s ON s.id = b.session_id
          WHERE b.local_date = ?1
          GROUP BY b.normalized_domain ORDER BY 3 DESC LIMIT 12",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([local_date], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Aggregate app usage for a local date, falling back to the daily rollup.
pub fn top_applications(conn: &Connection, local_date: &str) -> Vec<(String, String, i64, i64)> {
    let raw = raw_top_applications(conn, local_date);
    if !raw.is_empty() {
        return raw;
    }
    // Fall back to stored rollup JSON.
    let json: Option<String> = conn
        .query_row(
            "SELECT top_apps_json FROM daily_summaries WHERE local_date = ?1",
            [local_date],
            |r| r.get(0),
        )
        .ok();
    let Some(json) = json else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else { return vec![] };
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    Some((
                        o.get("process_name")?.as_str()?.to_string(),
                        o.get("category")?.as_str()?.to_string(),
                        o.get("total_ms")?.as_i64()?,
                        o.get("sessions")?.as_i64()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Aggregate top domains for a local date, falling back to the daily rollup.
pub fn top_websites(conn: &Connection, local_date: &str) -> Vec<(String, i64, i64)> {
    let raw = raw_top_websites(conn, local_date);
    if !raw.is_empty() {
        return raw;
    }
    let json: Option<String> = conn
        .query_row(
            "SELECT top_sites_json FROM daily_summaries WHERE local_date = ?1",
            [local_date],
            |r| r.get(0),
        )
        .ok();
    let Some(json) = json else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else { return vec![] };
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    Some((
                        o.get("domain")?.as_str()?.to_string(),
                        o.get("visits")?.as_i64()?,
                        o.get("total_ms")?.as_i64()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Hourly network totals in LOCAL hours (index = hour 0..24): `(sent, recv)`.
/// Stored timestamps are UTC, so group by the localtime hour. For rolled-up days
/// (raw snapshots purged) we fall back to `daily_summaries.hourly_network_json`.
pub fn network_hourly(conn: &Connection, local_date: &str) -> Vec<(i64, i64)> {
    let mut out = vec![(0i64, 0i64); 24];
    let sql = "SELECT CAST(strftime('%H', datetime(recorded_at, 'localtime')) AS INTEGER),
                      SUM(bytes_sent), SUM(bytes_received)
                 FROM network_snapshots WHERE local_date = ?1
                GROUP BY strftime('%H', datetime(recorded_at, 'localtime'))";
    let mut any = false;
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map([local_date], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for r in rows.flatten() {
                if (r.0 as usize) < 24 {
                    out[r.0 as usize] = (r.1, r.2);
                    any = true;
                }
            }
        }
    }
    if any {
        return out;
    }

    // Fallback: read the rollup's stored hourly network breakdown.
    let json: Option<String> = conn
        .query_row(
            "SELECT hourly_network_json FROM daily_summaries WHERE local_date = ?1",
            [local_date],
            |r| r.get(0),
        )
        .ok();
    if let Some(json) = json {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    let h = item.get("hour").and_then(|x| x.as_i64()).unwrap_or(0) as usize;
                    if h < 24 {
                        out[h] = (
                            item.get("sent").and_then(|x| x.as_i64()).unwrap_or(0),
                            item.get("recv").and_then(|x| x.as_i64()).unwrap_or(0),
                        );
                    }
                }
            }
        }
    }
    out
}

/// Sessions for the timeline: `(process_name, category, start_time, duration_ms)`.
pub fn session_timeline(conn: &Connection, local_date: &str) -> Vec<(String, String, String, i64)> {
    let mut stmt = match conn.prepare(
        "SELECT process_name, category, start_time, COALESCE(duration_ms,0)
           FROM app_sessions WHERE local_date = ?1 AND end_time IS NOT NULL
          ORDER BY start_time",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([local_date], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// A single setting value, if present.
pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).ok()
}

/// Look up a user-overridden category for a process, if any (M2).
pub fn category_for_process(conn: &Connection, process_name: &str) -> Option<String> {
    conn.query_row(
        "SELECT category FROM app_categories WHERE process_name = ?1",
        [process_name],
        |r| r.get(0),
    )
    .ok()
}

/// All excluded apps.
pub fn list_exclusions(conn: &Connection) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT process_name FROM excluded_apps ORDER BY excluded_at") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |r| r.get::<_, String>(0));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Current pause state (from settings `is_paused`).
pub fn get_paused(conn: &Connection) -> bool {
    get_setting(conn, "is_paused").map(|v| v == "true").unwrap_or(false)
}

/// All excluded domains.
pub fn list_excluded_domains(conn: &Connection) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT domain FROM excluded_domains") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |r| r.get::<_, String>(0));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}
