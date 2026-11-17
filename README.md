# Vigil

Vigil is a lightweight, privacy-first Windows 11 digital-activity monitor. It records
your screen time, apps, websites, and network usage — entirely on your own PC. No
cloud, no telemetry.

## Components

| Crate | Role |
| --- | --- |
| `src-service` | Background service (tray icon, tracking engine, SQLite persistence, IPC) |
| `src-tauri` | Tauri v2 dashboard UI (talks to the service over a named pipe) |
| `frontend` | React + Vite UI that is embedded into the Tauri shell |

## Features

- App / window / project-level screen-time tracking (incl. IDE → project detection)
- Domain-based productivity & focus scoring
- Deep-work / flow detection (45-min uninterrupted rule)
- Daily app/site usage limits with threshold nudges
- Multi-monitor attribution
- Battery / energy impact estimation
- Network byte accounting (interface counters)
- Daily CSV / PDF / Markdown (Obsidian) reports
- Self-update via GitHub Releases (Settings → Updates)

## Build

```powershell
.\scripts\setup.ps1          # verify toolchain + install frontend deps
.\scripts\build-all.ps1      # cargo workspace + frontend
.\scripts\package-release.ps1  # release binaries into .\dist
```

NSIS installer: compile `scripts\Vigil.nsi` with [NSIS 3.x](https://nsis.sourceforge.io)
(place `vigil-service.exe` and `vigil-dashboard.exe` beside it).

## Data

Everything is stored in `%LOCALAPPDATA%\Vigil\data.db` (SQLite, WAL). Reports
export to `%LOCALAPPDATA%\Vigil\reports\`.

## License

MIT
