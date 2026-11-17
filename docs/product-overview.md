# Vigil — Digital Activity Monitor

> *Vigil* (from **vigilance** — quietly watchful, lightweight, and dedicated to digital wellbeing)
> is a native Windows 11 application that watches what you actually do on your PC and turns it
> into honest, private, easy-to-understand insights about your screen time, apps, and sites.

## What it is
A **two-process desktop app**:

| Process | What it does | Runs |
|---|---|---|
| `vigil-service.exe` | Continuously records activity in the background (foreground app, idle/lock/sleep, browser tabs, network). Owns a tray icon. | 24/7 from login |
| `vigil-dashboard.exe` | A frameless, polished UI to browse and analyze the data. | On demand (from the tray) |

They talk over a **secure local pipe** — nothing leaves your machine.

## What it tracks
- **Apps** — how long you spend in each application (by process name + category).
- **Websites** — the domains you browse (YouTube, Kick, GitHub, …). Privacy-safe: only domains, not full URLs. Tab switches are detected via native UI Automation.
- **Screen time** — split into **Active** (typing/clicking, or watching/reading in a browser) and **Passive** (idle-but-on-screen). Total screen time is the headline metric.
- **Idle / Locked / Sleeping** — time your PC sat unused, locked, or asleep.
- **Network** — how much you upload/download, per hour.
- **Daily insights** — a focus score, the hourly pattern of your day, and category breakdowns.

## Key screens

| Screen | Shows |
|---|---|
| **Dashboard** | Activity rings (screen time vs goal, deep work, active ratio), category doughnut, hourly screen chart, top apps & sites, focus score |
| **Applications** | Ranked list of apps with usage bars and sessions |
| **Websites** | Ranked domains with time + visit counts |
| **Network** | Upload/download totals + hourly traffic chart |
| **Timeline** | Your day as a session flow |
| **Reports** | CSV export of any day |
| **Settings** | Pause monitoring, idle threshold, exclusions, backup, theme, and dashboard logging toggle |

## Privacy & design philosophy
- **100% local** — data lives in `%LOCALAPPDATA%\Vigil\data.db`. No cloud, no telemetry.
- Only the **service** writes the database (single-writer), the dashboard reads it.
- The service targets **≤15 MB RAM**; the dashboard uses **0 MB** when closed.
- Native per-tab detection is done in-app via Windows UI Automation — no browser extension needed.
- **Immersive shield**: fullscreen video/games and active audio never drop to idle (screen time keeps counting).
- **Controller support**: Xbox/PS controllers register as active input.
- **Anti-cheat aware**: games that block process queries are still identified via a process snapshot.
- **Clean timeline**: sub-second micro-sessions are merged; VLC/MPC titles are normalized.
- **Low footprint**: WAL checkpointing + working-set trimming keep the service small.

## Current status
Implemented across the full plan: tracking engine, data correctness + crash recovery, secure IPC, tray + auto-start, browser/network modules, and the React + Tauri dashboard (light/dark, hover-rich charts).
