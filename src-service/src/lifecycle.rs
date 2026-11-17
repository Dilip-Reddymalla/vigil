//! Lifecycle helpers: auto-start registration.
//!
//! Two methods:
//! - **Startup folder** (`shell:startup`) — no admin needed; best default for a
//!   per-user tray service. Used when admin elevation is unavailable.
//! - **Task Scheduler** (`schtasks /Create ... /SC ONLOGON`) — more robust, but
//!   requires an elevated prompt.

use anyhow::{bail, Context, Result};
use std::process::Command;

const TASK_NAME: &str = "Vigil Service";

/// Path to the per-user Windows Startup folder (e.g. `AppData\...\Startup`).
fn startup_dir() -> Result<std::path::PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let p = std::path::PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        return Ok(p);
    }
    bail!("APPDATA not set");
}

/// Register the service to start automatically at logon (per-user, no admin).
pub fn install_autostart() -> Result<()> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    let dir = startup_dir()?;
    std::fs::create_dir_all(&dir).context("create Startup folder")?;
    let link = dir.join("Vigil Service.cmd");

    // A small `.cmd` that starts the service hidden, tolerant of moves.
    let content = format!("@echo off\nstart \"\" \"{}\"\n", exe.display());
    std::fs::write(&link, content).context("write Startup shortcut")?;

    Ok(())
}

/// Remove the Startup-folder autostart entry.
pub fn remove_autostart() -> Result<()> {
    let dir = startup_dir()?;
    let link = dir.join("Vigil Service.cmd");
    if link.exists() {
        std::fs::remove_file(&link).context("remove Startup shortcut")?;
    }
    Ok(())
}

/// Optional: register as an on-logon Task Scheduler task (needs admin). Returns
/// `Ok(false)` if elevation is unavailable, without failing the caller.
pub fn install_autostart_task() -> Result<bool> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    let task_run = format!("\"{}\"", exe.display());
    let status = Command::new("schtasks")
        .args([
            "/Create",
            "/F",
            "/TN",
            TASK_NAME,
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/TR",
            &task_run,
        ])
        .status()
        .context("run schtasks /Create")?;

    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_name_is_stable() {
        assert_eq!(TASK_NAME, "Vigil Service");
    }
}
