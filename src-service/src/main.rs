//! Vigil service entry point.
#![windows_subsystem = "windows"]
//!
//! - Phase 1: window/input/power/session trackers → transition logs.
//! - Phase 2: SQLite persistence, heartbeat, session accounting, crash recovery.

use std::process::exit;
use vigil_service::{config::Config, trackers};

fn main() {
    // Lifecycle subcommands: `--install-autostart` / `--remove-autostart`.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--install-autostart") {
        // Per-user Startup folder (works without admin).
        if let Err(e) = vigil_service::lifecycle::install_autostart() {
            eprintln!("autostart install failed: {e:#}");
            exit(1);
        }
        println!("Vigil auto-start registered (Startup folder).");
        // Best-effort admin task (silently ignored if not elevated).
        match vigil_service::lifecycle::install_autostart_task() {
            Ok(true) => println!("Task Scheduler task also registered."),
            _ => println!("Task Scheduler skipped (not elevated / unavailable)."),
        }
        return;
    }
    if args.iter().any(|a| a == "--remove-autostart") {
        if let Err(e) = vigil_service::lifecycle::remove_autostart() {
            eprintln!("autostart remove failed: {e:#}");
            exit(1);
        }
        println!("Vigil auto-start removed.");
        return;
    }

    if let Err(e) = run() {
        eprintln!("vigil-service fatal: {e:#}");
        exit(1);
    }
}

/// Enforce a single service instance per user. Creates a named mutex; if another
/// instance already holds it, we return `false` so the caller exits quietly. The
/// handle is intentionally leaked for the process lifetime (it is never freed).
fn acquire_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let sid = std::process::id().to_string(); // fallback name
    let sid = std::env::var("USERNAME").unwrap_or(sid);
    let name = format!("Local\\VigilService_{sid}");
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();

    // SAFETY: CreateMutexW with a fixed name; we leak the handle on purpose.
    let h = unsafe { CreateMutexW(std::ptr::null_mut(), 1, wide.as_ptr()) };
    if h.is_null() {
        return true; // couldn't create; don't block startup
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return false; // another instance is running
    }
    true
}

fn run() -> anyhow::Result<()> {
    // Single-instance guard: exit quietly if another service is already running.
    if !acquire_single_instance() {
        eprintln!("vigil-service: another instance is already running; exiting.");
        return Ok(());
    }

    // Detach from the launch console so closing the terminal/console does NOT
    // terminate the service. It then only exits via the tray "Exit" action.
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }

    let cfg = Config::default();

    // Open DB (with migrations + startup recovery) and register the global engine.
    let engine = vigil_service::init_engine()?;

    // Log the version at startup for diagnostics (M4).
    vigil_service::log::raw(&serde_json::json!({
        "event": "startup",
        "version": vigil_service::engine::Engine::VERSION,
        "pid": std::process::id(),
    }));

    // Startup housekeeping: close orphaned active sessions + retention purge.
    {
        let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
        let _ = guard.housekeep();
    }

    // Apply persisted idle threshold (default 20 min) to the runtime config.
    {
        let guard = engine.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(v) = guard.get_setting("idle_threshold_seconds").as_str() {
            if let Ok(n) = v.parse::<u32>() {
                cfg.set_idle_threshold(n);
            }
        }
    }

    // Hot-reload the idle threshold when it changes via IPC (M7): the engine's
    // set_setting will update the shared atomic through this config.
    {
        let cfg = cfg.clone();
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("vigil-config-sync".to_string())
            .spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let guard = engine.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(v) = guard.get_setting("idle_threshold_seconds").as_str() {
                    if let Ok(n) = v.parse::<u32>() {
                        cfg.set_idle_threshold(n);
                    }
                }
            })
            .expect("spawn config-sync thread");
    }

    // Periodic housekeeping (every 6h) — closes strays and prunes old data.
    {
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("vigil-housekeep".to_string())
            .spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
                let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
                let _ = guard.housekeep();
            })
            .expect("spawn housekeep thread");
    }

    // Dedicated heartbeat timer (every 30s) — independent of event flow, so the
    // heartbeat never goes stale even if no tracker events arrive.
    {
        use std::time::Duration;
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("vigil-heartbeat".to_string())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(30));
                let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
                guard.refresh_heartbeat();
            })
            .expect("spawn heartbeat thread");
    }

    // Daily usage-limit nudge checker (every 60s).
    {
        use std::time::Duration;
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("vigil-limits".to_string())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(60));
                let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
                guard.limits_nudge();
                guard.record_battery();
            })
            .expect("spawn limits thread");
    }

    // Midnight splitter — waits until local midnight, then splits the active session.
    {
        use std::time::Duration;
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("vigil-midnight".to_string())
            .spawn(move || {
                loop {
                    let now = chrono::Local::now();
                    // Next local midnight (00:00 tomorrow). DST-safe: if the wall-clock
                    // time is ambiguous, pick the earliest local interpretation (M5).
                    let tomorrow = now.date_naive() + chrono::Duration::days(1);
                    let next = tomorrow
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(chrono::Local)
                        .earliest()
                        .unwrap_or(now + chrono::Duration::hours(24));
                    let wait_ms = (next - now).num_milliseconds().max(1_000);
                    std::thread::sleep(Duration::from_millis(wait_ms as u64));
                    let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
                    guard.roll_at_midnight();
                }
            })
            .expect("spawn midnight thread");
    }

    // 1. Foreground-window hook (requires the message loop on this thread).
    let hook = trackers::window::install()?;

    // 2. Input poller on a dedicated thread (also drives heartbeat cadence).
    let input_thread = trackers::input::spawn(cfg.clone());

    // 2b. Network poller (30s cadence).
    let _network_thread = trackers::network::spawn();

    // 3. Named-pipe JSON-RPC server on its own thread.
    std::thread::Builder::new()
        .name("vigil-ipc".to_string())
        .spawn(|| {
            let _ = vigil_service::ipc::server::run();
        })
        .expect("spawn ipc server thread");

    // 3. Hidden-window message loop for lock/sleep/monitor — blocks until quit.
    trackers::power::run_message_loop(&cfg)?;

    // Cleanup on WM_QUIT.
    trackers::window::uninstall(hook);
    let _ = input_thread.join();

    Ok(())
}
