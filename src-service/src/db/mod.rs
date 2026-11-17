//! Database module — the service is the **sole writer** (see `AGENTS.md`).
//!
//! Phase 2 wires the SQLite (WAL) writer: schema migrations, single-row heartbeat,
//! app sessions, and idle/non-screen periods. Phase 3 adds the IPC read path.

pub mod browser;
pub mod domain_categories;
pub mod energy;
pub mod heartbeat;
pub mod idle;
pub mod insights;
pub mod limits;
pub mod migrations;
pub mod network;
pub mod pdf;
pub mod report;
pub mod rollup;
pub mod schema;
pub mod session;
pub mod settings;
pub mod summary;
pub mod trends;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// The service-owned SQLite database.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the Vigil database and apply pending migrations.
    pub fn open() -> Result<Self> {
        let dir = data_dir().context("resolve Vigil data directory")?;
        std::fs::create_dir_all(&dir).context("create Vigil data directory")?;
        let path = dir.join("data.db");
        let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
        configure(&conn)?;
        let db = Self { conn };
        migrations::apply(&db.conn)?;
        Ok(db)
    }

    /// Access to the underlying connection (single writer).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Current database size in bytes (page_count × page_size).
    pub fn file_size(&self) -> rusqlite::Result<i64> {
        let pages: i64 = self.conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let page_size: i64 = self.conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok(pages * page_size)
    }
}

#[cfg(test)]
impl Database {
    /// Test-only in-memory database (same schema/migrations, no disk I/O).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        configure(&conn)?;
        let db = Self { conn };
        migrations::apply(&db.conn)?;
        Ok(db)
    }
}

/// `%LOCALAPPDATA%\Vigil`
fn data_dir() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into());
    Ok(PathBuf::from(base).join("Vigil"))
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(())
}
