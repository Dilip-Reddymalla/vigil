//! Vigil dashboard Rust crate.

/// Version reported to the service via IPC.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "tauri")]
pub mod db;
#[cfg(feature = "tauri")]
pub mod service;

/// Tauri commands bridging the frontend to `vigil-service` over the named pipe.
#[cfg(feature = "tauri")]
pub mod commands {
    use serde_json::{json, Value};

    use super::service::ServiceClient;

    #[tauri::command]
    pub fn get_dashboard(local_date: String) -> Result<Value, String> {
        // M6: try a direct read-only SQLite read first (heavier/historical);
        // fall back to IPC if the DB or row isn't available.
        if let Some(summary) = super::db::summary(&local_date) {
            return Ok(summary);
        }
        ServiceClient::call("dashboard.get_summary", Some(json!({ "local_date": local_date })))
    }

    #[tauri::command]
    pub fn get_status() -> Result<Value, String> {
        ServiceClient::call("service.status", None)
    }

    #[tauri::command]
    pub fn set_paused(paused: bool) -> Result<bool, String> {
        let method = if paused { "monitoring.pause" } else { "monitoring.resume" };
        ServiceClient::call(method, None).map(|_| paused)
    }

    #[tauri::command]
    pub fn set_setting(key: String, value: String, value_type: String) -> Result<bool, String> {
        ServiceClient::call(
            "settings.set",
            Some(json!({ "key": key, "value": value, "value_type": value_type })),
        )
        .map(|_| true)
    }

    #[tauri::command]
    pub fn add_exclusion(process_name: String) -> Result<bool, String> {
        ServiceClient::call("exclusions.add_app", Some(json!({ "process_name": process_name })))
            .map(|_| true)
    }

    #[tauri::command]
    pub fn backup_db(target_path: String) -> Result<bool, String> {
        ServiceClient::call("database.backup", Some(json!({ "target_path": target_path })))
            .map(|_| true)
    }

    #[tauri::command]
    pub fn export_report(
        anchor: String,
        target_path: String,
        granularity: String,
        format: String,
    ) -> Result<String, String> {
        let v = ServiceClient::call(
            "reports.export",
            Some(json!({
                "anchor": anchor,
                "target_path": target_path,
                "granularity": granularity,
                "format": format,
            })),
        )?;
        Ok(v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string())
    }

    #[tauri::command]
    pub fn applications_top(local_date: String) -> Result<Value, String> {
        ServiceClient::call("applications.top", Some(json!({ "local_date": local_date })))
    }

    #[tauri::command]
    pub fn websites_top(local_date: String) -> Result<Value, String> {
        ServiceClient::call("websites.top", Some(json!({ "local_date": local_date })))
    }

    #[tauri::command]
    pub fn network_hourly(local_date: String) -> Result<Value, String> {
        ServiceClient::call("network.hourly", Some(json!({ "local_date": local_date })))
    }

    #[tauri::command]
    pub fn timeline_sessions(local_date: String) -> Result<Value, String> {
        ServiceClient::call("timeline.sessions", Some(json!({ "local_date": local_date })))
    }

    #[tauri::command]
    pub fn settings_get(key: String) -> Result<Value, String> {
        ServiceClient::call("settings.get", Some(json!({ "key": key })))
    }

    #[tauri::command]
    pub fn system_name() -> Result<Value, String> {
        ServiceClient::call("system.name", None)
    }

    // ---- Updates (GitHub Releases) ----

    const DEFAULT_USER_AGENT: &str = "Vigil-Dashboard";
    /// Default update feed (used when the Settings fields are left empty).
    const DEFAULT_UPDATE_OWNER: &str = "Dilip-Reddymalla";
    const DEFAULT_UPDATE_REPO: &str = "vigil";

    /// Resolve the feed owner/repo from the command args, falling back to the
    /// baked-in defaults when empty.
    fn update_feed<'a>(owner: &'a str, repo: &'a str) -> (&'a str, &'a str) {
        let o = if owner.trim().is_empty() { DEFAULT_UPDATE_OWNER } else { owner.trim() };
        let r = if repo.trim().is_empty() { DEFAULT_UPDATE_REPO } else { repo.trim() };
        (o, r)
    }

    /// Fetch the newest **native** release (tag `v<num>...`, i.e. NOT the
    /// `python-dashboard-*` stream) from the GitHub Releases API. Each stream must
    /// update from its own releases — the native dashboard must never pull the
    /// python-dashboard release (which GitHub marks as "Latest").
    fn gh_latest(owner: &str, repo: &str) -> Result<Value, String> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=100");
        let resp = ureq::get(&url)
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", DEFAULT_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API request failed: {e}"))?;
        let mut text = String::new();
        use std::io::Read;
        resp.into_reader()
            .read_to_string(&mut text)
            .map_err(|e| format!("read GitHub response: {e}"))?;
        let releases: Value = serde_json::from_str(&text).map_err(|e| format!("bad GitHub response: {e}"))?;
        let arr = releases.as_array().ok_or("unexpected GitHub response shape")?;

        // Pick the newest native release (tag starts with "v" + a digit, and is
        // not the python-dashboard stream).
        let mut best: Option<(&str, &Value)> = None;
        for rel in arr {
            let tag = rel.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
            let is_native = tag.starts_with('v')
                && tag[1..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if !is_native {
                continue;
            }
            let ver = tag.trim_start_matches('v');
            match best {
                None => best = Some((ver, rel)),
                Some((cur, _)) => {
                    if is_newer(ver, cur) {
                        best = Some((ver, rel));
                    }
                }
            }
        }
        best.map(|(_, rel)| rel.clone()).ok_or("no native (v*) release found".into())
    }

    /// Parse a version string into leading numeric parts (e.g. "v0.1.3" → [0,1,3]).
    fn version_parts(v: &str) -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    }

    /// `a > b` when `a` is a newer version.
    fn is_newer(a: &str, b: &str) -> bool {
        let pa = version_parts(a);
        let pb = version_parts(b);
        for (x, y) in pa.iter().zip(pb.iter()) {
            if x != y {
                return x > y;
            }
        }
        pa.len() > pb.len()
    }

    fn update_state_dir() -> std::path::PathBuf {
        let base = std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into());
        std::path::PathBuf::from(base).join("Vigil").join("updates")
    }

    /// Locally installed version — stored after a successful update; initially the
    /// compiled APP_VERSION.
    fn installed_version() -> String {
        let f = update_state_dir().join("installed_version.txt");
        std::fs::read_to_string(f)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| super::APP_VERSION.to_string())
    }

    fn download_to(url: &str, dest: &std::path::Path) -> Result<(), String> {
        let resp = ureq::get(url)
            .set("User-Agent", DEFAULT_USER_AGENT)
            .call()
            .map_err(|e| format!("download failed: {e}"))?;
        let mut buf = Vec::new();
        use std::io::Read;
        resp.into_reader()
            .read_to_end(&mut buf)
            .map_err(|e| format!("read body: {e}"))?;
        use std::io::Write;
        let mut file = std::fs::File::create(dest).map_err(|e| format!("create file: {e}"))?;
        file.write_all(&buf).map_err(|e| format!("write file: {e}"))?;
        Ok(())
    }

    /// `update.check` — query the latest GitHub release and report whether it's
    /// newer than the installed version. `owner`/`repo` come from Settings.
    #[tauri::command]
    pub fn update_check(owner: String, repo: String) -> Result<Value, String> {
        let (owner, repo) = update_feed(&owner, &repo);
        let release = gh_latest(owner, repo)?;
        let remote = release
            .get("tag_name")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim_start_matches('v')
            .to_string();
        let local = installed_version();
        let has_update = !remote.is_empty() && is_newer(&remote, &local);
        let asset_names: Vec<String> = release
            .get("assets")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(json!({
            "installed": local,
            "latest": remote,
            "has_update": has_update,
            "assets": asset_names,
        }))
    }

    /// `update.apply` — download the service + dashboard assets for the latest
    /// release, then hand a detached PowerShell script that swaps the binaries
    /// (stops the service → copies → restarts) and relaunches the dashboard.
    /// The UI calls `app_exit` right after this returns.
    #[tauri::command]
    pub fn update_apply(owner: String, repo: String) -> Result<bool, String> {
        let (owner, repo) = update_feed(&owner, &repo);
        let release = gh_latest(owner, repo)?;
        let version = release
            .get("tag_name")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim_start_matches('v')
            .to_string();
        if version.is_empty() {
            return Err("release has no version tag".into());
        }

        let install_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or("cannot resolve installation directory")?;

        let dl_dir = update_state_dir().join(&version);
        std::fs::create_dir_all(&dl_dir).map_err(|e| e.to_string())?;

        // Download every asset in the release (service, dashboard, icons, …).
        let assets = release.get("assets").and_then(|a| a.as_array()).cloned().unwrap_or_default();
        if assets.is_empty() {
            return Err("release has no downloadable assets".into());
        }
        let mut copies: Vec<(String, String)> = Vec::new(); // (src, dst)
        let mut has_service = false;
        let mut has_dashboard = false;
        for a in &assets {
            let Some(name) = a.get("name").and_then(|n| n.as_str()) else { continue };
            let Some(url) = a.get("browser_download_url").and_then(|u| u.as_str()) else { continue };
            let lname = name.to_ascii_lowercase();
            if lname.contains("vigil-service.exe") {
                has_service = true;
            } else if lname.contains("vigil-dashboard.exe") {
                has_dashboard = true;
            }
            // Nest downloads under the version dir; install flat into install_dir.
            let src = dl_dir.join(name);
            let dst = install_dir.join(name);
            download_to(url, &src)?;
            copies.push((src.display().to_string(), dst.display().to_string()));
        }
        if !has_service {
            return Err("release is missing the service binary".into());
        }
        if !has_dashboard {
            return Err("release is missing the dashboard binary".into());
        }

        let state_dir = update_state_dir();
        let state_file = state_dir.join("installed_version.txt");
        let script_path = state_dir.join("apply.ps1");

        let copy_lines = copies
            .iter()
            .map(|(src, dst)| format!("Copy-Item '{src}' '{dst}' -Force\n"))
            .collect::<String>();

        let script = format!(
            r#"
$ErrorActionPreference = 'Stop'
Start-Sleep -Milliseconds 800
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Process -Name vigil-dashboard -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {{ Start-Sleep -Milliseconds 200 }}
Stop-Process -Name vigil-service -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 600
{copy_lines}New-Item -ItemType Directory -Force -Path '{state_dir}' | Out-Null
Set-Content -Path '{state_file}' -Value '{version}'
Start-Process '{install_dir}\vigil-service.exe' -WindowStyle Hidden
Start-Sleep -Milliseconds 1500
Start-Process '{install_dir}\vigil-dashboard.exe'
"#,
            copy_lines = copy_lines,
            state_dir = state_dir.display(),
            state_file = state_file.display(),
            version = version,
            install_dir = install_dir.display(),
        );

        std::fs::write(&script_path, script).map_err(|e| e.to_string())?;

        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script_path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("failed to start updater: {e}"))?;
        Ok(true)
    }

    /// `app.exit` — close the dashboard (the updater script relaunches it).
    #[tauri::command]
    pub fn app_exit(app: tauri::AppHandle) {
        app.exit(0);
    }

    /// `overview.compare` - current vs previous period metrics for the
    /// Improvements page.
    #[tauri::command]
    pub fn overview_compare(period: String) -> Result<Value, String> {
        ServiceClient::call("overview.compare", Some(json!({ "period": period })))
    }

    /// `overview.week` — full metrics for a user-selected week.
    #[tauri::command]
    pub fn overview_week(anchor: String) -> Result<Value, String> {
        ServiceClient::call("overview.week", Some(json!({ "anchor": anchor })))
    }

    #[tauri::command]
    pub fn exclusions_list() -> Result<Value, String> {
        ServiceClient::call("exclusions.list", None)
    }

    #[tauri::command]
    pub fn database_retention(keep_days: i64) -> Result<Value, String> {
        ServiceClient::call("database.retention", Some(json!({ "keep_days": keep_days })))
    }

    #[tauri::command]
    pub fn database_retention_get() -> Result<Value, String> {
        ServiceClient::call("database.retention.get", None)
    }

    /// Open the reports folder in Explorer (creates it if missing).
    #[tauri::command]
    pub fn open_report_folder() -> Result<bool, String> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA not set")?;
        let dir = std::path::PathBuf::from(base).join("Vigil").join("reports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::process::Command::new("explorer.exe")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Append a line to `%LOCALAPPDATA%\Vigil\dashboard.log` so the UI's
    /// activity/errors are persisted and inspectable (not just console).
    #[tauri::command]
    pub fn dashboard_log(level: String, message: String) {
        use std::io::Write;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("t={}", d.as_secs()))
            .unwrap_or_else(|_| "t=?".into());
        let line = format!("{stamp} [{level}] {message}\n");
        if let Some(appdata) = std::env::var_os("LOCALAPPDATA") {
            let dir = std::path::PathBuf::from(appdata).join("Vigil");
            let _ = std::fs::create_dir_all(&dir);
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("dashboard.log"))
            {
                let _ = f.write_all(line.as_bytes());
            }
        }
        // Also mirror to console for dev visibility.
        println!("[Vigil][dashboard] {line}");
    }
}

/// Run the native Tauri shell (feature `tauri`).
#[cfg(feature = "tauri")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::get_status,
            commands::set_paused,
            commands::set_setting,
            commands::add_exclusion,
            commands::backup_db,
            commands::export_report,
            commands::applications_top,
            commands::websites_top,
            commands::network_hourly,
            commands::timeline_sessions,
            commands::settings_get,
            commands::system_name,
            commands::exclusions_list,
            commands::dashboard_log,
            commands::open_report_folder,
            commands::database_retention,
            commands::database_retention_get,
            commands::update_check,
            commands::update_apply,
            commands::app_exit,
            commands::overview_compare,
            commands::overview_week
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!APP_VERSION.is_empty());
    }
}
