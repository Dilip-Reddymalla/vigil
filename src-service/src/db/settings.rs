//! Settings & exclusions persistence.

use rusqlite::{params, Connection};

/// Upsert a typed setting value.
pub fn upsert_setting(conn: &Connection, key: &str, value: &str, value_type: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value, value_type) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            value_type = excluded.value_type,
            updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        params![key, value, value_type],
    )?;
    Ok(())
}

/// Add an excluded app (idempotent).
pub fn add_exclusion(conn: &Connection, process_name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO excluded_apps (process_name) VALUES (?1)",
        params![process_name],
    )?;
    Ok(())
}
