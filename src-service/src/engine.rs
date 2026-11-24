//! The **Engine** binds the state machine to persistence.
//!
//! It owns the `MonitorState`, the SQLite `Database`, the currently-open app session,
//! and any in-progress non-screen period. Every telemetry event is applied here
//! under a single lock so DB writes are serialized (single-writer rule).

use anyhow::Context;
use chrono::{DateTime, Local, Utc};

use crate::categorizer::categorize;
use crate::db::{self, Database};
use crate::events::TrackingEvent;
use crate::log;
use crate::state::{MonitorState, Transition};

/// An in-progress non-screen period (locked/sleep/monitor_off/idle).
struct NonScreenPeriod {
    period_type: &'static str,
    source: &'static str,
    start: DateTime<Utc>,
    local_date: String,
}

pub struct Engine {
    state: MonitorState,
    db: Database,
    session: Option<db::session::ActiveSession>,
    non_screen: Option<NonScreenPeriod>,
    started_at: std::time::Instant,
    paused: bool,
    /// In-memory set of excluded app process names (loaded at startup).
    excluded_apps: std::collections::HashSet<String>,
    /// In-memory set of excluded domains.
    excluded_domains: std::collections::HashSet<String>,
    /// Last finalized session `(uuid, process_name, end_time)` — used to merge
    /// sub-second micro-sessions back into it (Phase 3 / de-noising).
    recent_closed: Option<(String, String, DateTime<Utc>)>,
    /// Per-limit last-notified threshold bucket (90/100) to avoid duplicate nudges.
    limit_notified: std::collections::HashMap<String, i64>,
}

impl Engine {
    /// Open the database and run startup crash recovery.
    pub fn new(db: Database) -> anyhow::Result<Self> {
        let (last_alive, _active) = db::heartbeat::read(db.conn())?;
        let (recovered, seconds) = db::session::recover_open(db.conn(), last_alive.as_deref())?;
        if recovered > 0 {
            log::raw(&serde_json::json!({
                "event": "recovery",
                "recovered_sessions": recovered,
                "reconciled_seconds": seconds,
            }));
        }
        let excluded_apps: std::collections::HashSet<String> =
            crate::db::insights::list_exclusions(db.conn()).into_iter().collect();
        let excluded_domains: std::collections::HashSet<String> = crate::db::insights::list_excluded_domains(db.conn()).into_iter().collect();

        Ok(Self {
            state: MonitorState::new(),
            db,
            session: None,
            non_screen: None,
            started_at: std::time::Instant::now(),
            paused: false,
            excluded_apps,
            excluded_domains,
            recent_closed: None,
            limit_notified: std::collections::HashMap::new(),
        })
    }

    pub fn state(&self) -> &MonitorState {
        &self.state
    }

    /// Apply an event: update state, drive sessions/periods/heartbeat, return transitions.
    pub fn handle(&mut self, ev: TrackingEvent) -> Vec<Transition> {
        // Track active duration by wall-clock time (C4), not just 5s ticks.
        if let TrackingEvent::ActivityPoll { state, .. } = &ev {
            if let Some(s) = self.session.as_mut() {
                s.total_ticks += 1;
                if *state == crate::state::ActivityState::Active {
                    s.active_ticks += 1;
                    if s.last_active_start.is_none() {
                        s.last_active_start = Some(Utc::now());
                    }
                } else {
                    if let Some(start) = s.last_active_start.take() {
                        s.active_duration_ms += (Utc::now() - start).num_milliseconds().max(0);
                    }
                }
            }
        }

        // Phase 1 (fullscreen/audio shield): if an immersive fullscreen/presentation
        // state is active and the display is on, never downgrade to Idle. This keeps
        // screen time counting while watching movies/streams or playing exclusive-D3D
        // games without input.
        let ev = match &ev {
            TrackingEvent::ActivityPoll {
                state: crate::state::ActivityState::Idle,
                last_input_seconds,
                timestamp,
            } if self.state.display().is_on() && crate::trackers::fullscreen::should_keep_active() => {
                TrackingEvent::ActivityPoll {
                    state: crate::state::ActivityState::Passive,
                    last_input_seconds: *last_input_seconds,
                    timestamp: *timestamp,
                }
            }
            other => other.clone(),
        };

        let transitions = self.state.apply(ev);
        for t in &transitions {
            self.apply_transition(t);
        }
        self.write_heartbeat();

        if self.session.is_some() {
            log::raw(&serde_json::json!({
                "event": "engine_state",
                "state": self.state.snapshot_json(),
                "open_session": self.session.as_ref().map(|s| s.process_name.clone()),
                "transitions": transitions.len(),
            }));
        }

        transitions
    }

    fn apply_transition(&mut self, t: &Transition) {
        match t.kind {
            "screen" => {
                if t.to == "none" {
                    self.end_session(t.ts);
                    self.begin_non_screen(t.ts);
                } else {
                    self.end_non_screen(t.ts);
                    self.open_session_if_needed(t.ts);
                }
            }
            "window" => {
                if !self.state.is_screen_time() {
                    return;
                }
                // Only switch apps when the *process* changes. Title-only changes
                // (navigation, progress updates) keep the same session open — this
                // avoids fragmenting time into seconds-long sessions.
                if let Some(fg) = self.state.foreground() {
                    // Ignore transient shell overlays (Alt-Tab / empty explorer) entirely.
                    if is_shell_noise(&fg.process_name, &fg.title) {
                        return;
                    }
                    if let Some(s) = self.session.as_mut() {
                        if s.process_name.eq_ignore_ascii_case(&fg.process_name) {
                            // C11: if the window handle differs, this is a different
                            // window (e.g. split-screen) of the same app → switch.
                            if s.hwnd != 0 && fg.hwnd != 0 && s.hwnd != fg.hwnd {
                                // fallthrough to close/reopen new session
                            } else {
                                // Debounce: remember the last (process,title,win) + time.
                                s.last_foreground = Some((
                                    fg.process_name.clone(),
                                    fg.title.clone(),
                                    t.ts.timestamp_millis(),
                                ));
                                s.window_title =
                                    crate::trackers::media::clean_media_title(&s.process_name, &fg.title);
                                s.hwnd = fg.hwnd;
                                // Same window (e.g. browser tab/title change) → record a
                                // new visit when the extracted domain changes (C10).
                                if s.is_browser {
                                    let local_date = s.local_date.clone();
                                    if let Some(adapter) =
                                        crate::trackers::browser::adapter_for_process(&s.process_name)
                                    {
                                        if !adapter.is_incognito(&fg.title) {
                                            if let Some(url) = adapter.extract_url_with_hwnd(&fg.title, fg.hwnd) {
                                                record_visit(
                                                    self.db.conn(),
                                                    s,
                                                    &adapter.browser_name(),
                                                    &url,
                                                    t.ts,
                                                    &local_date,
                                                    &self.excluded_domains,
                                                );
                                            }
                                        }
                                    }
                                }
                                return;
                            }
                        }
                    }
                }
                self.end_session(t.ts);
                self.open_session_if_needed(t.ts);
            }
            _ => {}
        }
    }

    fn open_session_if_needed(&mut self, ts: DateTime<Utc>) {
        if self.paused || self.session.is_some() {
            return;
        }
        let Some(fg) = self.state.foreground().cloned() else {
            return;
        };
        if fg.process_name.is_empty() || is_shell_noise(&fg.process_name, &fg.title) {
            return;
        }

        let process_name = fg.process_name;

        // Never track our own processes (they would pollute the report).
        if matches!(
            process_name.to_ascii_lowercase().as_str(),
            "vigil-service.exe" | "vigil-service" | "vigil-dashboard.exe" | "vigil-dashboard"
        ) {
            return;
        }

        // Honor user exclusions (loaded at startup from the excluded_apps table).
        if self.excluded_apps.contains(&process_name.to_ascii_lowercase()) {
            return;
        }

        let title = fg.title;
        // Prefer a user-overridden category (M2), else the built-in categorizer.
        let category = crate::db::insights::category_for_process(self.db.conn(), &process_name)
            .unwrap_or_else(|| categorize(&process_name).to_string());
        let browser = crate::trackers::browser::adapter_for_process(&process_name);
        let is_browser = browser.is_some();
        let local_date = Local::now().format("%Y-%m-%d").to_string();

        let mut session = db::session::ActiveSession {
            uuid: uuid::Uuid::new_v4().to_string(),
            process_name: process_name.clone(),
            executable_path: None,
            window_title: crate::trackers::media::clean_media_title(&process_name, &title),
            category,
            project_name: crate::trackers::project::project_from_title(&process_name, &title),
            monitor_index: crate::trackers::monitor::monitor_index_for_window(fg.hwnd),
            is_browser,
            start_time: ts,
            local_date: local_date.clone(),
            hwnd: fg.hwnd,
            db_id: 0,
            active_ticks: 0,
            total_ticks: 0,
            last_active_start: None,
            active_duration_ms: 0,
            last_visit_domain: None,
            last_foreground: None,
        };

        match db::session::insert(self.db.conn(), &session) {
            Ok(id) => session.db_id = id,
            Err(e) => {
                log::raw(&serde_json::json!({
                    "event": "error", "op": "session_insert", "error": e.to_string()
                }));
                return;
            }
        }

        // Record an app launch (once per foreground change to a new process).
        if let Err(e) = db::session::insert_launch(
            self.db.conn(),
            &session.process_name,
            None,
            &ts,
            &local_date,
        ) {
            log::raw(&serde_json::json!({
                "event": "error", "op": "launch_insert", "error": e.to_string()
            }));
        }

        // Record the opening site (if the browser exposes one). Skip incognito.
        if let Some(adapter) = browser.as_ref() {
            if !adapter.is_incognito(&title) {
                if let Some(url) = adapter.extract_url_with_hwnd(&title, fg.hwnd) {
                    record_visit(
                        self.db.conn(),
                        &mut session,
                        &adapter.browser_name(),
                        &url,
                        ts,
                        &local_date,
                        &self.excluded_domains,
                    );
                }
            }
        }

        log::raw(&serde_json::json!({
            "event": "session_start",
            "uuid": session.uuid,
            "app": session.process_name,
            "category": session.category,
        }));
        self.session = Some(session);
    }

    fn end_session(&mut self, ts: DateTime<Utc>) {
        if let Some(mut s) = self.session.take() {
            // Close any in-progress active interval (C4).
            if let Some(start) = s.last_active_start.take() {
                s.active_duration_ms += (ts - start).num_milliseconds().max(0);
            }
            let total_ms = (ts - s.start_time).num_milliseconds().max(0);
            let duration = (ts - s.start_time).num_milliseconds();

            // Phase 3: sub-second session collapsing. If this session lasted < 2s,
            // is the same process as the previously finalized session, and that
            // session ended < 2s ago → fold the time into it and drop this row.
            const MIN_SESSION_MS: i64 = 2000;
            if duration < MIN_SESSION_MS {
                // Clone the candidate before mutating self (avoid borrow conflict).
                let merge_with = self.recent_closed.as_ref().and_then(|(ruuid, rproc, rend)| {
                    if *rproc == s.process_name && (ts - *rend).num_milliseconds() < MIN_SESSION_MS {
                        Some((ruuid.clone(), rproc.clone(), *rend))
                    } else {
                        None
                    }
                });
                if let Some((ruuid, rproc, _rend)) = merge_with {
                    let _ = crate::db::session::extend_to_end(self.db.conn(), &ruuid, &ts);
                    let _ = crate::db::session::delete_session(self.db.conn(), &s.uuid);
                    self.recent_closed = Some((ruuid.clone(), rproc, ts));
                    log::raw(&serde_json::json!({
                        "event": "session_merged", "into": ruuid, "dropped": s.uuid
                    }));
                    return;
                }
            }

            let ratio = if total_ms > 0 {
                (s.active_duration_ms as f64 / total_ms as f64).clamp(0.0, 1.0)
            } else {
                1.0
            };
            match db::session::finalize(self.db.conn(), &s.uuid, &ts, "completed", ratio) {
                Ok(()) => {
                    log::raw(&serde_json::json!({
                        "event": "session_end", "uuid": s.uuid, "status": "completed"
                    }));
                    self.recent_closed = Some((s.uuid.clone(), s.process_name.clone(), ts));
                }
                Err(e) => log::raw(&serde_json::json!({
                    "event": "error", "op": "session_end", "error": e.to_string()
                })),
            }
        }
    }

    fn begin_non_screen(&mut self, ts: DateTime<Utc>) {
        if self.paused || self.non_screen.is_some() {
            return;
        }
        let (period_type, source) = classify_period(&self.state);
        self.non_screen = Some(NonScreenPeriod {
            period_type,
            source,
            start: ts,
            local_date: Local::now().format("%Y-%m-%d").to_string(),
        });
    }

    fn end_non_screen(&mut self, ts: DateTime<Utc>) {
        if let Some(p) = self.non_screen.take() {
            if let Err(e) = db::idle::insert(
                self.db.conn(),
                p.period_type,
                p.source,
                &p.start,
                &ts,
                &p.local_date,
            ) {
                log::raw(&serde_json::json!({
                    "event": "error", "op": "idle_insert", "error": e.to_string()
                }));
            }
        }
    }

    fn write_heartbeat(&mut self) {
        let now = Utc::now().to_rfc3339();
        let snapshot = db::heartbeat::HeartbeatSnapshot {
            last_alive_at: &now,
            system_state: self.state.system(),
            activity_state: self.state.activity(),
            display_state: self.state.display(),
            active_session: self.session.as_ref().map(|s| s.uuid.as_str()),
        };
        if let Err(e) = db::heartbeat::write(self.db.conn(), &snapshot) {
            log::raw(&serde_json::json!({
                "event": "error", "op": "heartbeat", "error": e.to_string()
            }));
        }
    }

    /// Public heartbeat tick used by the dedicated timer thread.
    pub fn refresh_heartbeat(&mut self) {
        self.write_heartbeat();
    }
}

/// `%LOCALAPPDATA%\Vigil\reports`
fn reports_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into());
    Ok(std::path::PathBuf::from(base).join("Vigil").join("reports"))
}

/// Resolve a report target path. Absolute paths are used as-is; empty/relative
/// paths are rooted under the reports directory with `default_name`.
fn resolve_report_target(target_path: &str, default_name: &str) -> anyhow::Result<std::path::PathBuf> {
    let target = if target_path.trim().is_empty() {
        reports_dir()?.join(default_name)
    } else {
        let p = std::path::PathBuf::from(target_path);
        if p.is_absolute() {
            p
        } else {
            reports_dir()?.join(p)
        }
    };
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).context("create reports directory")?;
    }
    Ok(target)
}

/// Insert a browser visit if the domain differs from the last recorded one for this
/// session (so tab/site switches register without fragmenting the session).
fn record_visit(
    conn: &rusqlite::Connection,
    session: &mut db::session::ActiveSession,
    browser_name: &str,
    url: &crate::trackers::browser::BrowserUrl,
    ts: DateTime<Utc>,
    local_date: &str,
    excluded_domains: &std::collections::HashSet<String>,
) {
    let same = session.last_visit_domain.as_deref() == Some(url.normalized_domain.as_str());
    if same {
        return;
    }
    // Honor excluded domains.
    if excluded_domains.contains(&url.normalized_domain) {
        return;
    }
    if let Err(e) = db::browser::insert_visit(
        conn,
        session.db_id,
        browser_name,
        &url.domain,
        &url.normalized_domain,
        &url.title,
        &ts,
        local_date,
    ) {
        log::raw(&serde_json::json!({
            "event": "error", "op": "visit_insert", "error": e.to_string()
        }));
        return;
    }
    session.last_visit_domain = Some(url.normalized_domain.clone());
    log::raw(&serde_json::json!({
        "event": "visit", "session": session.uuid, "domain": url.normalized_domain
    }));
}

/// Processes that are purely OS shell/overlay surfaces — not user activity.
/// NOTE: explorer.exe is handled separately (File Explorer is legit activity)
/// so it's not included here.
fn is_noise_process(process_name: &str) -> bool {
    let p = process_name.to_ascii_lowercase();
    matches!(
        p.as_str(),
        "shellhost.exe" | "searchhost.exe" | "startmenuexperiencehost.exe"
            | "applicationframehost.exe" | "textinputhost.exe" | "shellexperiencehost.exe"
            | "taskhostw.exe" | "dwm.exe" | "mmc.exe" | "runtimebroker.exe"
    )
}

/// True for transient Windows shell overlays we should not record as user activity
/// (Alt-Tab "Task Switching" UI, empty-title Explorer projections, etc).
fn is_shell_noise(process_name: &str, title: &str) -> bool {
    let p = process_name.to_ascii_lowercase();
    let t = title.to_ascii_lowercase();

    // Purely-overlay processes are always noise.
    if is_noise_process(process_name) {
        return true;
    }

    // explorer.exe: File Explorer with a real title is legit activity; only the
    // bare / shell-overlay variants are noise.
    if p.starts_with("explorer") {
        return t.is_empty()
            || t.contains("task switching")
            || t.contains("task view")
            || t.contains("program manager")
            || t.contains("system tray overflow")
            || t.contains("windows shell experience");
    }
    false
}

/// Classify why screen time stopped, prioritizing system-level causes.
fn classify_period(state: &MonitorState) -> (&'static str, &'static str) {
    use crate::state::{ActivityState, DisplayState, SystemState};
    match state.system() {
        SystemState::Locked => return ("locked", "session_lock"),
        SystemState::Sleeping => return ("sleep", "power_suspend"),
        SystemState::Unlocked => {}
    }
    match state.display() {
        DisplayState::Off => return ("monitor_off", "monitor_power"),
        DisplayState::On => {}
    }
    if state.activity() == ActivityState::Idle {
        ("idle", "input_timeout")
    } else {
        // Defensive fallback.
        ("idle", "unknown")
    }
}

/// Public API consumed by the JSON-RPC layer.
impl Engine {
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    /// Read access to the DB connection (for tests / read-only queries).
    pub fn db_conn(&self) -> &rusqlite::Connection {
        self.db.conn()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// `service.status` result.
    pub fn status(&self) -> serde_json::Value {
        let active_session = self.session.as_ref().map(|s| {
            serde_json::json!({
                "session_uuid": s.uuid,
                "process_name": s.process_name,
                "window_title": s.window_title,
                "category": s.category,
                "start_time": s.start_time.to_rfc3339(),
            })
        });
        let db_size = self.db.file_size().unwrap_or(0);
        serde_json::json!({
            "version": Self::VERSION,
            "uptime_seconds": self.started_at.elapsed().as_secs(),
            "system_state": self.state.system().as_str(),
            "activity_state": self.state.activity().as_str(),
            "display_state": self.state.display().as_str(),
            "active_session": active_session,
            "db_size_bytes": db_size,
            "is_paused": self.paused,
        })
    }

    /// `dashboard.get_summary` result.
    pub fn summary(&self, local_date: &str) -> serde_json::Value {
        crate::db::summary::daily(self.db.conn(), local_date)
    }

    /// `settings.set`.
    pub fn set_setting(&mut self, key: &str, value: &str, value_type: &str) -> anyhow::Result<()> {
        crate::db::settings::upsert_setting(self.db.conn(), key, value, value_type)?;
        Ok(())
    }

    /// `exclusions.add_app`.
    pub fn add_exclusion(&mut self, process_name: &str) -> anyhow::Result<()> {
        crate::db::settings::add_exclusion(self.db.conn(), process_name)?;
        // Keep the in-memory set in sync so the exclusion applies immediately (H7).
        self.excluded_apps.insert(process_name.to_ascii_lowercase());
        Ok(())
    }

    /// `database.backup` (SQLite-aware snapshot).
    pub fn backup(&mut self, target: &str) -> anyhow::Result<()> {
        self.db.conn().execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// `reports.export_csv` — write a CSV report for a local date.
    ///
    /// If `target_path` is empty or relative, the file is written under
    /// `%LOCALAPPDATA%\Vigil\reports\`. Returns the absolute path written.
    pub fn export_csv(&mut self, local_date: &str, target_path: &str) -> anyhow::Result<String> {
        let csv = crate::db::report::sessions_csv(self.db.conn(), local_date)?;
        let target = resolve_report_target(target_path, &format!("{local_date}-report.csv"))?;
        std::fs::write(&target, csv).context("write report csv")?;
        Ok(target.to_string_lossy().into_owned())
    }

    /// `reports.export` — export a daily/weekly/monthly report as CSV or PDF.
    ///
    /// `granularity`: "daily" | "weekly" | "monthly"; `format`: "csv" | "pdf".
    pub fn export_report(
        &mut self,
        granularity: &str,
        anchor: &str,
        format: &str,
        target_path: &str,
    ) -> anyhow::Result<String> {
        use crate::db::report::{build_report, Granularity};

        let gran = Granularity::parse(granularity)
            .ok_or_else(|| anyhow::anyhow!("unknown granularity: {granularity}"))?;
        let report = build_report(self.db.conn(), gran, anchor)
            .ok_or_else(|| anyhow::anyhow!("invalid anchor date: {anchor}"))?;

        let (ext, bytes) = match format.to_ascii_lowercase().as_str() {
            "pdf" => ("pdf", crate::db::report::report_pdf(&report)),
            _ => ("csv", crate::db::report::report_csv(&report).into_bytes()),
        };

        let default_name = format!("{}-report.{}", report.rows.first().map(|r| r.label.as_str()).unwrap_or(anchor), ext);
        let target = resolve_report_target(target_path, &default_name)?;
        std::fs::write(&target, bytes).context("write report")?;
        Ok(target.to_string_lossy().into_owned())
    }

    /// `reports.export_md` — export a daily/weekly/monthly report as Markdown
    /// (ready to drop into an Obsidian vault/journal). `format` is ignored.
    pub fn export_md(
        &mut self,
        granularity: &str,
        anchor: &str,
        target_path: &str,
    ) -> anyhow::Result<String> {
        use crate::db::report::{build_report, Granularity};

        let gran = Granularity::parse(granularity)
            .ok_or_else(|| anyhow::anyhow!("unknown granularity: {granularity}"))?;
        let report = build_report(self.db.conn(), gran, anchor)
            .ok_or_else(|| anyhow::anyhow!("invalid anchor date: {anchor}"))?;

        let md = crate::db::report::report_md(&report);
        let default_name = format!("{}-report.md", report.rows.first().map(|r| r.label.as_str()).unwrap_or(anchor));
        let target = resolve_report_target(target_path, &default_name)?;
        std::fs::write(&target, md.as_bytes()).context("write report md")?;
        Ok(target.to_string_lossy().into_owned())
    }

    /// `applications.top`
    pub fn apps(&self, local_date: &str) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = crate::db::insights::top_applications(self.db.conn(), local_date)
            .into_iter()
            .map(|(p, c, d, s)| serde_json::json!({ "process_name": p, "category": c, "total_ms": d, "sessions": s }))
            .collect();
        serde_json::json!(arr)
    }

    /// `websites.top`
    pub fn sites(&self, local_date: &str) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = crate::db::insights::top_websites(self.db.conn(), local_date)
            .into_iter()
            .map(|(d, v, t)| serde_json::json!({ "domain": d, "visits": v, "total_ms": t }))
            .collect();
        serde_json::json!(arr)
    }

    /// `network.hourly`
    pub fn network_hourly(&self, local_date: &str) -> serde_json::Value {
        let hourly = crate::db::insights::network_hourly(self.db.conn(), local_date);
        let arr: Vec<serde_json::Value> = hourly
            .into_iter()
            .enumerate()
            .map(|(h, (sent, recv))| serde_json::json!({ "hour": h, "sent": sent, "recv": recv }))
            .collect();
        serde_json::json!(arr)
    }

    /// `timeline.sessions`
    pub fn timeline(&self, local_date: &str) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = crate::db::insights::session_timeline(self.db.conn(), local_date)
            .into_iter()
            .map(|(p, c, s, d)| serde_json::json!({ "process_name": p, "category": c, "start_time": s, "duration_ms": d }))
            .collect();
        serde_json::json!(arr)
    }

    /// `settings.get`
    pub fn get_setting(&self, key: &str) -> serde_json::Value {
        crate::db::insights::get_setting(self.db.conn(), key)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null)
    }

    /// `system.name` — display name shown in the dashboard. Uses the user's
    /// custom `display_name` setting when set, otherwise the Windows computer
    /// name (`COMPUTERNAME`).
    pub fn system_name(&self) -> String {
        if let Some(n) = crate::db::insights::get_setting(self.db.conn(), "display_name") {
            let t = n.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "This PC".into())
    }

    /// `overview.compare` — current vs previous period (week/month) for the
    /// Improvements page. Falls back to an empty JSON when the period can't be
    /// resolved (shouldn't happen with a valid today).
    pub fn compare_periods(&self, period: &str) -> serde_json::Value {
        crate::db::trends::compare(self.db.conn(), period)
            .unwrap_or_else(|| serde_json::json!({ "verdict": "mixed", "reasons": [] }))
    }

    /// `overview.week` — full metrics for a user-selected week (Mon–Sun).
    pub fn week_detail(&self, anchor: &str) -> serde_json::Value {
        crate::db::trends::week_detail(self.db.conn(), anchor)
            .unwrap_or_else(|| serde_json::json!({ "period": "week_detail", "metrics": {}, "daily": [] }))
    }

    /// `exclusions.list`
    pub fn exclusions(&self) -> serde_json::Value {
        serde_json::json!(crate::db::insights::list_exclusions(self.db.conn()))
    }

    /// `database.retention` — purge raw events older than `keep_days`.
    pub fn set_retention(&mut self, keep_days: i64) -> anyhow::Result<()> {
        crate::db::settings::upsert_setting(
            self.db.conn(),
            "retention_days",
            &keep_days.to_string(),
            "integer",
        )?;
        let (s, v, n) = crate::db::session::purge_old(self.db.conn(), keep_days)?;
        crate::log::raw(&serde_json::json!({
            "event": "retention_purge",
            "keep_days": keep_days,
            "sessions": s, "visits": v, "network": n
        }));
        Ok(())
    }

    /// Read current retention setting (default 90 days).
    pub fn retention_days(&self) -> i64 {
        crate::db::insights::get_setting(self.db.conn(), "retention_days")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(90)
    }

    /// `database.merge_session` — extend an existing (uuid) session to a new end
    /// time, merging across the local-midnight boundary.
    pub fn merge_session_to(&mut self, uuid: &str, end: &DateTime<Utc>) -> anyhow::Result<()> {
        crate::db::session::extend_to_end(self.db.conn(), uuid, end)?;
        Ok(())
    }

    /// Split the active session at local midnight: close the current one (assigned
    /// to its start day) and reopen a fresh session for the new day.
    pub fn roll_at_midnight(&mut self) {
        let now = Utc::now();
        if self.session.is_some() && self.state.is_screen_time() {
            self.end_session(now);
            self.open_session_if_needed(now);
        }
        // Also write a heartbeat so recovery knows we were alive at the boundary.
        self.write_heartbeat();
    }

    /// Close orphaned `active` sessions that are stuck, and purge raw data older
    /// than the retention window.
    ///
    /// Orphaned sessions are closed using the last heartbeat time (so their real
    /// duration is preserved), falling back to "now" if no heartbeat exists — never
    /// zeroing their duration. This is distinct from startup `recover_open`.
    pub fn housekeep(&mut self) -> anyhow::Result<()> {
        let now = Utc::now();
        let conn = self.db.conn();
        let last_alive = crate::db::heartbeat::read(conn)?.0;
        let orphaned: Vec<(String, String)> = {
            let mut stmt = conn.prepare("SELECT session_uuid, start_time FROM app_sessions WHERE status='active'")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for (uuid, _start) in orphaned {
            let end = last_alive
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now);
            crate::db::session::finalize(conn, &uuid, &end, "recovered", 1.0)?;
        }

        // 2. Retention purge.
        let keep = self.retention_days();
        crate::db::session::purge_old(conn, keep)?;

        // 3. Roll up past days outside the raw-keep window, then drop their raw rows.
        let keep_raw = self.raw_keep_days();
        let (days_rolled, rows_removed) = crate::db::rollup::consolidate_past_days(conn, keep_raw)?;
        if days_rolled > 0 || rows_removed > 0 {
            crate::log::raw(&serde_json::json!({
                "event": "rollup", "days_rolled": days_rolled, "rows_removed": rows_removed
            }));
        }

        // 4. Compaction: checkpoint + truncate the WAL, then trim the working set
        //    so the background service stays small (Phase 10).
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        unsafe {
            let process = windows_sys::Win32::System::Threading::GetCurrentProcess();
            let _ = windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx(
                process, usize::MAX, usize::MAX, 0,
            );
        }

        Ok(())
    }

    /// Check configured usage limits against today's usage; emit a `limit_nudge`
    /// event the first time a limit crosses 90% and again at 100% (Phase 7.1).
    pub fn limits_nudge(&mut self) {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let limits = crate::db::limits::read_limits(self.db.conn());
        let statuses = crate::db::limits::compute_limits(self.db.conn(), &date, &limits);
        for st in statuses {
            let key = format!("{}:{}", st.kind, st.name);
            let bucket = if st.pct >= 100 { 100 } else if st.pct >= 90 { 90 } else { 0 };
            if bucket > 0 {
                let prev = self.limit_notified.get(&key).copied().unwrap_or(0);
                if prev < bucket {
                    crate::log::raw(&serde_json::json!({
                        "event": "limit_nudge",
                        "kind": st.kind, "name": st.name,
                        "used_ms": st.used_ms, "limit_ms": st.limit_ms, "pct": st.pct
                    }));
                    self.limit_notified.insert(key, bucket);
                }
            }
        }
    }

    /// Record the current battery state (Phase 9). Harmless if on AC or unmetered.
    pub fn record_battery(&mut self) {
        let Some((ac, pct, flag)) = crate::trackers::battery::battery_status() else {
            return;
        };
        let date = Local::now().format("%Y-%m-%d").to_string();
        let ts = Utc::now().to_rfc3339();
        let _ = crate::db::energy::record_battery(self.db.conn(), &date, &ts, ac, pct, flag);
    }

    /// `energy.status` — current battery + day's drain estimate + per-app share.
    pub fn energy_status(&self, local_date: &str) -> serde_json::Value {
        let current = crate::trackers::battery::battery_status()
            .map(|(ac, pct, flag)| serde_json::json!({
                "on_ac": ac == 1, "percent": pct, "flag": flag
            }));
        let drain = crate::db::energy::battery_drain_pct_per_hour(self.db.conn(), local_date);
        let apps: Vec<serde_json::Value> = crate::db::energy::energy_by_app(self.db.conn(), local_date)
            .into_iter()
            .map(|(p, ms, d)| serde_json::json!({ "process_name": p, "total_ms": ms, "drain_pct": d }))
            .collect();
        serde_json::json!({
            "current": current,
            "drain_pct_per_hour": drain,
            "apps": apps,
        })
    }

    /// How many recent days remain in raw form. Default 0 → the previous day is
    /// consolidated on startup (only "today" stays raw for live UI).
    pub fn raw_keep_days(&self) -> i64 {
        crate::db::insights::get_setting(self.db.conn(), "raw_keep_days")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TrackingEvent;
    use crate::state::ActivityState;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn engine() -> Engine {
        let db = crate::db::Database::open_in_memory().unwrap();
        Engine::new(db).unwrap()
    }

    /// Advancing fake clock so sessions have real durations (>2s, avoiding merge).
    fn ts(step: i64) -> DateTime<Utc> {
        use std::sync::atomic::{AtomicI64, Ordering};
        static T0: AtomicI64 = AtomicI64::new(0);
        let base = if T0.load(Ordering::Relaxed) == 0 {
            T0.store(Utc::now().timestamp_millis(), Ordering::Relaxed);
            T0.load(Ordering::Relaxed)
        } else {
            T0.load(Ordering::Relaxed)
        };
        DateTime::from_timestamp_millis(base + step * 3000).unwrap()
    }

    #[test]
    fn session_lifecycle_open_close_reopen() {
        crate::trackers::fullscreen::set_shields_enabled(false);
        let mut e = engine();

        // Foreground established → session opens (screen time is on by default).
        e.handle(TrackingEvent::WindowChange {
            process_id: 1,
            process_name: "code.exe".into(),
            title: "Editor".into(),
            hwnd: 0,
            timestamp: ts(1),
        });
        assert!(e.session.is_some(), "session should open after foreground event");

        // Lock → session closes, a `locked` period begins.
        e.handle(TrackingEvent::SessionLock { timestamp: ts(2) });
        assert!(e.session.is_none(), "session should close on lock");
        assert!(e.non_screen.is_some());

        // Unlock → period flushed, session reopens (different process to avoid merge).
        e.handle(TrackingEvent::SessionUnlock { timestamp: ts(3) });
        e.handle(TrackingEvent::WindowChange {
            process_id: 2,
            process_name: "brave.exe".into(),
            title: "YouTube".into(),
            hwnd: 0,
            timestamp: ts(4),
        });
        assert!(e.session.is_some(), "session should reopen on unlock");

        // Idle → screen stops, session closes, idle period recorded.
        e.handle(TrackingEvent::ActivityPoll {
            state: ActivityState::Idle,
            last_input_seconds: 400,
            timestamp: ts(5),
        });
        assert!(e.session.is_none());

        // Verify persisted state.
        let conn = e.db.conn();
        let sessions: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_sessions", [], |r| r.get(0))
            .unwrap();
        assert!(sessions >= 2, "expected >=2 sessions recorded, got {sessions}");
        // All closed sessions must be `completed` (no orphan 'active').
        let active: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_sessions WHERE status='active'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(active, 0);

        let idle_periods: i64 = conn
            .query_row("SELECT COUNT(*) FROM idle_periods", [], |r| r.get(0))
            .unwrap();
        assert!(idle_periods >= 1);
    }
}
