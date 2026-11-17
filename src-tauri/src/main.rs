//! Vigil dashboard binary entry point.

// GUI-subsystem app: no console window is created when launched (including from
// the tray), so no extra terminal pops up.
#![windows_subsystem = "windows"]

fn main() {
    #[cfg(feature = "tauri")]
    {
        vigil_dashboard_lib::run();
    }

    #[cfg(not(feature = "tauri"))]
    {
        println!(
            "vigil-dashboard {} — native Tauri shell disabled. Build with `--features tauri` on an MSVC toolchain.",
            vigil_dashboard_lib::APP_VERSION
        );
    }
}
