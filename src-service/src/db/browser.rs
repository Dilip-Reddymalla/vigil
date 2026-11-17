//! Browser visit persistence.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// Record a browser visit for a session. Uses the session's DB row id directly
/// (H2 — no subquery lookup).
pub fn insert_visit(
    conn: &Connection,
    session_id: i64,
    browser_name: &str,
    domain: &str,
    normalized_domain: &str,
    page_title: &str,
    start_time: &DateTime<Utc>,
    local_date: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO browser_visits
            (session_id, browser_name, url, domain, normalized_domain, page_title,
             start_time, local_date)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
        params![
            session_id,
            browser_name,
            domain,
            normalized_domain,
            page_title,
            start_time.to_rfc3339(),
            local_date
        ],
    )?;
    Ok(())
}
