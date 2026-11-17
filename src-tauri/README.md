# `src-tauri` — Vigil Dashboard Shell (Phase 5)

This is the **Tauri v2** host for the Vigil dashboard.

## Role
- Opens SQLite **read-only** (`SQLITE_OPEN_READONLY`) for heavy historical queries.
- **Everything else** goes over the Named Pipe (`\\.\pipe\vigil_{USER_SID}`,
  JSON-RPC 2.0) to `vigil-service`.
- Frameless window, Mica backdrop, `data-tauri-drag-region` title bar.
- Closes with `app.exit(0)` (0 MB RAM when not viewing). The background service is
  never touched by closing the dashboard.

## IPC contract (frontend command → this crate → service)
| Command | Service method | Purpose |
|---|---|---|
| `get_dashboard` | `dashboard.get_summary` | daily summary |
| `get_status` | `service.status` | service health |
| `set_paused` | `monitoring.pause/resume` | stop/start recording |
| `set_setting` | `settings.set` | persist a setting |
| `add_exclusion` | `exclusions.add_app` | exclude an app |
| `backup_db` | `database.backup` | SQLite-safe backup |
| `export_report` | `reports.export_csv` | CSV report export |

The React frontend (`../frontend`) calls these through `@tauri-apps/api`'s `invoke`.

## Build note
A full `tauri build` requires the **MSVC** toolchain + **WebView2** runtime and a
valid `icons/icon.ico` (bundler target). The workspace deliberately keeps this crate
light (no Tauri deps gated on) so `cargo build --workspace` stays green on the
GNU/MinGW toolchain; enable/compile the native shell on an MSVC-capable machine.
