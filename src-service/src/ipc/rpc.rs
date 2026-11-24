//! JSON-RPC 2.0 method dispatch.

use serde_json::{json, Value};

use crate::engine::Engine;

/// Well-known RPC error codes.
#[derive(Debug)]
pub enum RpcError {
    Parse,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    Database(String),
    ServiceBusy,
    Unauthorized,
}

impl RpcError {
    pub fn code(&self) -> i64 {
        match self {
            RpcError::Parse => -32700,
            RpcError::InvalidRequest => -32600,
            RpcError::MethodNotFound => -32601,
            RpcError::InvalidParams => -32602,
            RpcError::Database(_) => -32000,
            RpcError::ServiceBusy => -32001,
            RpcError::Unauthorized => -32002,
        }
    }

    pub fn message(&self) -> String {
        match self {
            RpcError::Parse => "Parse error".into(),
            RpcError::InvalidRequest => "Invalid Request".into(),
            RpcError::MethodNotFound => "Method not found".into(),
            RpcError::InvalidParams => "Invalid params".into(),
            RpcError::Database(e) => format!("Database error: {e}"),
            RpcError::ServiceBusy => "Service busy".into(),
            RpcError::Unauthorized => "Unauthorized".into(),
        }
    }
}

/// Dispatch a method against the engine.
pub fn dispatch(engine: &mut Engine, method: &str, params: Option<&Value>) -> Result<Value, RpcError> {
    crate::log::raw(&serde_json::json!({
        "event": "rpc_request", "method": method, "params": params
    }));
    let result = dispatch_inner(engine, method, params);
    match &result {
        Ok(v) => crate::log::raw(&serde_json::json!({
            "event": "rpc_response", "method": method, "result": v,
            "result_bytes": serde_json::to_string(v).map(|s| s.len()).unwrap_or(0),
        })),
        Err(e) => crate::log::raw(&serde_json::json!({
            "event": "rpc_error", "method": method, "code": e.code(), "message": e.message()
        })),
    }
    result
}

fn dispatch_inner(engine: &mut Engine, method: &str, params: Option<&Value>) -> Result<Value, RpcError> {
    match method {
        "service.status" => Ok(engine.status()),
        "dashboard.get_summary" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.summary(date))
        }
        "monitoring.pause" => {
            engine.set_paused(true);
            Ok(json!(true))
        }
        "monitoring.resume" => {
            engine.set_paused(false);
            Ok(json!(true))
        }
        "settings.set" => {
            let key = str_param(params, "key")?;
            let value = str_param(params, "value")?;
            let value_type = str_param(params, "value_type")?;
            engine
                .set_setting(key, value, value_type)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!(true))
        }
        "exclusions.add_app" => {
            let process_name = str_param(params, "process_name")?;
            engine
                .add_exclusion(process_name)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!(true))
        }
        "database.backup" => {
            let target = str_param(params, "target_path")?;
            engine
                .backup(target)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!(true))
        }
        "reports.export_csv" => {
            let local_date = str_param(params, "local_date")?;
            let target = str_param(params, "target_path")?;
            let written = engine
                .export_csv(local_date, target)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!({ "path": written }))
        }
        "reports.export" => {
            let granularity = str_param(params, "granularity")?;
            let anchor = str_param(params, "anchor")?;
            let format = str_param(params, "format")?;
            let target = str_param(params, "target_path")?;
            let written = engine
                .export_report(granularity, anchor, format, target)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!({ "path": written }))
        }
        "reports.export_md" => {
            let granularity = str_param(params, "granularity")?;
            let anchor = str_param(params, "anchor")?;
            let target = str_param(params, "target_path")?;
            let written = engine
                .export_md(granularity, anchor, target)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!({ "path": written }))
        }
        "energy.status" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.energy_status(date))
        }
        "overview.compare" => {
            let period = str_param(params, "period").unwrap_or_else(|_| "week".into());
            Ok(engine.compare_periods(&period))
        }
        "overview.week" => {
            let anchor = match str_param(params, "anchor") {
                Ok(a) => a.to_string(),
                Err(_) => chrono::Local::now().format("%Y-%m-%d").to_string(),
            };
            Ok(engine.week_detail(&anchor))
        }
        "database.retention" => {
            let days: i64 = params
                .and_then(|p| p.get("keep_days"))
                .and_then(|v| v.as_i64())
                .unwrap_or(90);
            engine
                .set_retention(days)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!({ "retention_days": days }))
        }
        "database.retention.get" => Ok(json!({ "retention_days": engine.retention_days() })),
        "database.merge_session" => {
            let uuid = str_param(params, "uuid")?;
            let end = str_param(params, "end")?;
            let end_dt = chrono::DateTime::parse_from_rfc3339(end)
                .map(|d| d.with_timezone(&chrono::Utc))
                .map_err(|_| RpcError::InvalidParams)?;
            engine
                .merge_session_to(uuid, &end_dt)
                .map_err(|e| RpcError::Database(e.to_string()))?;
            Ok(json!(true))
        }
        "applications.top" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.apps(date))
        }
        "websites.top" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.sites(date))
        }
        "network.hourly" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.network_hourly(date))
        }
        "timeline.sessions" => {
            let date = str_param(params, "local_date")?;
            Ok(engine.timeline(date))
        }
        "settings.get" => {
            let key = str_param(params, "key")?;
            Ok(engine.get_setting(key))
        }
        "system.name" => Ok(json!(engine.system_name())),
        "exclusions.list" => Ok(engine.exclusions()),
        _ => Err(RpcError::MethodNotFound),
    }
}

fn str_param<'a>(params: Option<&'a Value>, name: &str) -> Result<&'a str, RpcError> {
    params
        .and_then(|p| p.get(name))
        .and_then(|v| v.as_str())
        .ok_or(RpcError::InvalidParams)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use chrono::Utc;

    #[test]
    fn unknown_method_returns_error() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut engine = Engine::new(db).unwrap();
        assert!(matches!(
            dispatch(&mut engine, "does.not.exist", None),
            Err(RpcError::MethodNotFound)
        ));
    }

    #[test]
    fn status_returns_service_info() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut engine = Engine::new(db).unwrap();
        let value = dispatch(&mut engine, "service.status", None).unwrap();
        assert_eq!(value["version"].as_str().unwrap(), env!("CARGO_PKG_VERSION"));
        assert_eq!(value["is_paused"].as_bool(), Some(false));
    }

    #[test]
    fn pause_resume_toggles() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut engine = Engine::new(db).unwrap();
        dispatch(&mut engine, "monitoring.pause", None).unwrap();
        assert!(engine.is_paused());
        dispatch(&mut engine, "monitoring.resume", None).unwrap();
        assert!(!engine.is_paused());
    }

    #[test]
    fn exclusions_and_settings_persist() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut engine = Engine::new(db).unwrap();

        dispatch(
            &mut engine,
            "exclusions.add_app",
            Some(&json!({ "process_name": "KeePass.exe" })),
        )
        .unwrap();
        dispatch(
            &mut engine,
            "settings.set",
            Some(&json!({ "key": "idle_threshold_seconds", "value": "300", "value_type": "integer" })),
        )
        .unwrap();

        let n: i64 = engine
            .db_conn()
            .query_row("SELECT COUNT(*) FROM excluded_apps WHERE process_name='KeePass.exe'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);

        let v: String = engine
            .db_conn()
            .query_row("SELECT value FROM settings WHERE key='idle_threshold_seconds'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, "300");
    }

    #[test]
    fn reports_export_csv_writes_file() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut engine = Engine::new(db).unwrap();

        // Seed one session row.
        let session = crate::db::session::ActiveSession {
            uuid: "csv-session".into(),
            process_name: "code.exe".into(),
            executable_path: None,
            window_title: "Editor".into(),
            category: "development".into(),
            project_name: None,
            monitor_index: 0,
            is_browser: false,
            start_time: Utc::now() - chrono::Duration::minutes(30),
            local_date: "2026-11-13".into(),
            hwnd: 0,
            db_id: 0,
            active_ticks: 100,
            total_ticks: 100,
            last_active_start: None,
            active_duration_ms: 0,
            last_visit_domain: None,
            last_foreground: None,
        };
        crate::db::session::insert(engine.db_conn(), &session).unwrap();

        let out = std::env::temp_dir().join("vigil_report_test.csv");
        let out_s = out.to_string_lossy().to_string();
        dispatch(
            &mut engine,
            "reports.export_csv",
            Some(&json!({ "local_date": "2026-11-13", "target_path": out_s })),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("start_time_local"));
        assert!(content.contains("code.exe"));
        let _ = std::fs::remove_file(&out);
    }
}
