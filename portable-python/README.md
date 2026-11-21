# Vigil python-dashboard 0.1.5 (Smart App Control friendly)

This branch/folder provides an alternative distribution of the Vigil **dashboard**
and **setup** that runs under trusted runtimes (Python + a web browser), so it is
**not blocked by Windows Smart App Control**. The core service (`vigil-service.exe`)
is unchanged — this only swaps the blocked pieces (the Tauri dashboard exe and the
NSIS installer) for scripted, browser-based equivalents.

## Why

Smart App Control (SAC) blocks unsigned `.exe` files. The `vigil-dashboard.exe`
and `Vigil-Setup-*.exe` are unsigned, so they are blocked on SAC machines.
Python and a normal browser are trusted by SAC, so running the dashboard through
them works without signing and without disabling SAC.

## What's here

| File | Role |
| --- | --- |
| `bridge_server.py` | Serves the UI on `http://127.0.0.1:8765` and relays RPC to the service's named pipe |
| `web/` | The compiled dashboard UI (same as the Tauri build, browser transport) |
| `install.py` | "Setup" replacement: deploys package, registers service autostart, writes the tray launcher |
| `bin/` *(optional)* | Put `vigil-service.exe` here to have the installer deploy it automatically |
| `dashboard-launcher.cmd` | Written by `install.py` into `%LOCALAPPDATA%\Vigil`; what the tray "Open Dashboard" runs |

## Install (on a SAC machine)

1. Install **Python 3.11+** from python.org (the official, signed installer).
2. Extract this package anywhere.
3. Run:
   ```powershell
   python install.py
   ```
4. Start the dashboard either from the tray **Open Dashboard**, or by running the
   generated `dashboard-launcher.cmd`.

## Tray integration

The service's tray "Open Dashboard" was extended (in this branch only) to check
for `dashboard-launcher.cmd` beside the service first, and run it instead of the
blocked `vigil-dashboard.exe`. So the tray button opens the browser dashboard on
SAC systems. If the launcher is absent, it falls back to the normal exe.

## Requirements / notes

- Python must be on `PATH` (the installer uses `pythonw`/`pyw`, falling back to `python`).
- The bridge is **stdlib-only** (ctypes for the named pipe, `http.server` for the UI).
  No unsigned third-party DLLs are used.
- Service must be running (the service itself is deployed/started by `install.py`
  if `bin/vigil-service.exe` is provided, or must already be installed).
- Tab/`pickFolder` in the browser build uses a prompt instead of a native folder
  dialog.

## Differences from the main branch

- `frontend/src/lib/api.ts` — transport swapped from Tauri `invoke` to an HTTP
  call to the local bridge.
- `frontend/src/lib/logger.ts` — file logging skipped in browser (no Tauri host).
- `frontend/src/lib/api.ts` — `pickFolder` uses a prompt.
- `src-service/src/tray.rs` — `open_dashboard()` supports the launcher override.
- `frontend/db` is rebuilt for the browser.

**This branch is separate. It does not change the `main` branch or the normal
release.**
