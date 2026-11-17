//! SQLite schema (canonical source of truth).
//!
//! This is the SQL used to create the Vigil database. Phase 1 only ships the
//! schema as data; Phase 2 will wire it into the service via `rusqlite`
//! (migration runner, single-writer access, WAL pragmas).

/// Connection pragmas applied on every open.
pub const PRAGMAS: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA cache_size = -2000;
";

/// Full schema DDL.
pub const SCHEMA_SQL: &str = r#"
-- 1. Migrations
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    description TEXT
);

-- 2. Heartbeat (single row)
CREATE TABLE IF NOT EXISTS heartbeat (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    last_alive_at   TEXT NOT NULL,
    system_state    TEXT NOT NULL,
    activity_state  TEXT NOT NULL,
    display_state   TEXT NOT NULL,
    active_session  TEXT,
    updated_at      TEXT NOT NULL
);
INSERT OR IGNORE INTO heartbeat (id, last_alive_at, system_state, activity_state, display_state, updated_at)
VALUES (1, '', 'unlocked', 'idle', 'on', '');

-- 3. App sessions
CREATE TABLE IF NOT EXISTS app_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_uuid    TEXT NOT NULL UNIQUE,
    process_name    TEXT NOT NULL,
    executable_path TEXT,
    window_title    TEXT,
    category        TEXT NOT NULL DEFAULT 'other',
    project_name    TEXT,
    monitor_index   INTEGER NOT NULL DEFAULT 0,
    is_browser      INTEGER NOT NULL DEFAULT 0,
    start_time      TEXT NOT NULL,
    end_time        TEXT,
    duration_ms     INTEGER,
    local_date      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',
    is_deep_work    INTEGER NOT NULL DEFAULT 0,
    active_ratio    REAL NOT NULL DEFAULT 1.0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 4. Browser visits
CREATE TABLE IF NOT EXISTS browser_visits (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id        INTEGER NOT NULL REFERENCES app_sessions(id) ON DELETE CASCADE,
    browser_name      TEXT NOT NULL,
    url               TEXT,
    domain            TEXT NOT NULL,
    normalized_domain TEXT NOT NULL,
    page_title        TEXT,
    start_time        TEXT NOT NULL,
    end_time          TEXT,
    duration_ms       INTEGER,
    local_date        TEXT NOT NULL,
    is_incognito      INTEGER NOT NULL DEFAULT 0
);

-- 5. Idle / non-screen periods
CREATE TABLE IF NOT EXISTS idle_periods (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    period_type      TEXT NOT NULL,
    detection_source TEXT NOT NULL,
    start_time       TEXT NOT NULL,
    end_time         TEXT,
    duration_ms      INTEGER,
    local_date       TEXT NOT NULL
);

-- 6. Network snapshots
CREATE TABLE IF NOT EXISTS network_snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    interface_name  TEXT NOT NULL,
    interface_luid  TEXT NOT NULL,
    interface_type  TEXT NOT NULL,
    bytes_sent      INTEGER NOT NULL,
    bytes_received  INTEGER NOT NULL,
    cumulative_sent INTEGER NOT NULL,
    cumulative_recv INTEGER NOT NULL,
    recorded_at     TEXT NOT NULL,
    local_date      TEXT NOT NULL
);

-- 7. App launches
CREATE TABLE IF NOT EXISTS app_launches (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    process_name    TEXT NOT NULL,
    executable_path TEXT,
    launched_at     TEXT NOT NULL,
    local_date      TEXT NOT NULL
);

-- 8. Daily summaries (derived)
CREATE TABLE IF NOT EXISTS daily_summaries (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    local_date            TEXT NOT NULL UNIQUE,
    total_screen_time_ms  INTEGER NOT NULL DEFAULT 0,
    total_active_time_ms  INTEGER NOT NULL DEFAULT 0,
    total_passive_time_ms INTEGER NOT NULL DEFAULT 0,
    total_idle_time_ms    INTEGER NOT NULL DEFAULT 0,
    total_locked_time_ms  INTEGER NOT NULL DEFAULT 0,
    total_sleep_time_ms   INTEGER NOT NULL DEFAULT 0,
    session_count         INTEGER NOT NULL DEFAULT 0,
    first_activity        TEXT,
    last_activity         TEXT,
    productivity_score    REAL NOT NULL DEFAULT 0.0,
    deep_work_sessions    INTEGER NOT NULL DEFAULT 0,
    deep_work_time_ms     INTEGER NOT NULL DEFAULT 0,
    app_switches          INTEGER NOT NULL DEFAULT 0,
    distraction_index     REAL NOT NULL DEFAULT 0.0,
    total_bytes_sent      INTEGER NOT NULL DEFAULT 0,
    total_bytes_received  INTEGER NOT NULL DEFAULT 0,
    top_apps_json         TEXT,
    top_sites_json        TEXT,
    hourly_breakdown_json TEXT,
    hourly_network_json    TEXT,
    goal_met              INTEGER NOT NULL DEFAULT 0,
    computed_at           TEXT NOT NULL
);

-- 9. User customization
CREATE TABLE IF NOT EXISTS app_categories (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    process_name TEXT NOT NULL UNIQUE,
    category     TEXT NOT NULL,
    display_name TEXT,
    icon_path    TEXT,
    is_user_set  INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    value_type  TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE TABLE IF NOT EXISTS excluded_apps (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    process_name TEXT NOT NULL UNIQUE,
    excluded_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE TABLE IF NOT EXISTS excluded_domains (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    domain      TEXT NOT NULL UNIQUE,
    excluded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE TABLE IF NOT EXISTS domain_categories (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    domain   TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS goal_streaks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    start_date  TEXT NOT NULL,
    end_date    TEXT,
    streak_days INTEGER NOT NULL DEFAULT 1,
    goal_type   TEXT NOT NULL,
    goal_value  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS battery_snapshots (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    local_date          TEXT NOT NULL,
    ts                  TEXT NOT NULL,
    ac_line_status      INTEGER NOT NULL, -- 0 = battery, 1 = AC
    battery_life_percent INTEGER NOT NULL,
    battery_flag         INTEGER NOT NULL DEFAULT 0
);

-- Indexes (local-date & lookup)
CREATE INDEX IF NOT EXISTS idx_sessions_local_date ON app_sessions(local_date);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON app_sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_process ON app_sessions(process_name);
CREATE INDEX IF NOT EXISTS idx_sessions_category ON app_sessions(category);
CREATE INDEX IF NOT EXISTS idx_visits_local_date ON browser_visits(local_date);
CREATE INDEX IF NOT EXISTS idx_visits_domain ON browser_visits(normalized_domain);
CREATE INDEX IF NOT EXISTS idx_visits_session ON browser_visits(session_id);
CREATE INDEX IF NOT EXISTS idx_idle_local_date ON idle_periods(local_date);
CREATE INDEX IF NOT EXISTS idx_idle_type ON idle_periods(period_type);
CREATE INDEX IF NOT EXISTS idx_network_local_date ON network_snapshots(local_date);
CREATE INDEX IF NOT EXISTS idx_network_luid ON network_snapshots(interface_luid);
CREATE INDEX IF NOT EXISTS idx_launches_local_date ON app_launches(local_date);
CREATE INDEX IF NOT EXISTS idx_daily_local_date ON daily_summaries(local_date);
"#;

/// The current schema version marker.
pub const SCHEMA_VERSION: i64 = 7;
