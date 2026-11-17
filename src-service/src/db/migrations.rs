//! Lightweight schema versioning using the `schema_migrations` table.

use anyhow::Result;
use rusqlite::{params, Connection};

use super::schema::{SCHEMA_SQL, SCHEMA_VERSION};

/// Apply all pending migrations (idempotent).
pub fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY,
            applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            description TEXT
        );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    if current < SCHEMA_VERSION {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(SCHEMA_SQL)?;

        // Schema upgrades for each version.
        let add_col_if_missing = |tx: &rusqlite::Transaction, table: &str, col: &str, ddl: &str| -> rusqlite::Result<()> {
            let n: i64 = tx.query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name='{col}'"),
                [],
                |r| r.get(0),
            )?;
            if n == 0 {
                tx.execute(ddl, [])?;
            }
            Ok(())
        };

        // v2: session_count
        add_col_if_missing(&tx, "daily_summaries", "session_count",
            "ALTER TABLE daily_summaries ADD COLUMN session_count INTEGER NOT NULL DEFAULT 0")?;
        // v3: hourly_network_json
        add_col_if_missing(&tx, "daily_summaries", "hourly_network_json",
            "ALTER TABLE daily_summaries ADD COLUMN hourly_network_json TEXT")?;
        // v4: domain_categories table created by SCHEMA_SQL (IF NOT EXISTS).
        // v5: app_sessions.project_name
        add_col_if_missing(&tx, "app_sessions", "project_name",
            "ALTER TABLE app_sessions ADD COLUMN project_name TEXT")?;
        // v6: app_sessions.monitor_index
        add_col_if_missing(&tx, "app_sessions", "monitor_index",
            "ALTER TABLE app_sessions ADD COLUMN monitor_index INTEGER NOT NULL DEFAULT 0")?;
        // v7: battery_snapshots table created by SCHEMA_SQL (IF NOT EXISTS).

        tx.execute(
            "INSERT INTO schema_migrations (version, description) VALUES (?1, ?2)",
            params![SCHEMA_VERSION, "schema v7: battery_snapshots"],
        )?;
        tx.commit()?;
    }

    // Seed default domain categories (idempotent).
    super::domain_categories::ensure_seeded(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn applies_schema_and_seeds_heartbeat() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).unwrap();

        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type = 'table' AND name IN
                    ('app_sessions','browser_visits','idle_periods','network_snapshots',
                     'daily_summaries','settings')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 6);

        let hb: i64 = conn
            .query_row("SELECT COUNT(*) FROM heartbeat WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(hb, 1);
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).unwrap();
        apply(&conn).unwrap();
        let n1: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        let n2: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        // Applying twice must not add rows; latest version present.
        assert_eq!(n1, n2);
        assert!(n1 <= SCHEMA_VERSION);
    }
}
