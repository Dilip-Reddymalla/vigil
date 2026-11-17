# Vigil Architecture

This document is a living summary of the Vigil architecture. It codifies the invariants from
the `vigil-dev` skill (`config/plugins/vigil`) and the implementation plan.

## 1. Two-Process Split

Vigil runs as two separate executables communicating over a Named Pipe:

| Process | Binary | Role | Lifetime |
|---|---|---|---|
| Service | `vigil-service.exe` | All monitoring, analysis, DB writes | 24/7 from login (Task Scheduler) |
| Dashboard | `vigil-dashboard.exe` | UI only — reads data, sends settings | On tray click; **fully exits** on window close |

Rules:
- Only `vigil-service` **writes** SQLite. The dashboard opens read-only (`SQLITE_OPEN_READONLY`)
  or queries via IPC.
- Closing the dashboard terminates the dashboard process (`app.exit(0)`), never the service.

## 2. Three Orthogonal State Dimensions

Screen time uses three independent dimensions. Each transitions on its own:

| Dimension | States | Source |
|---|---|---|
| SystemState | `Unlocked ⇄ Locked ⇄ Sleeping` | `WTSRegisterSessionNotification`, Power notifications |
| ActivityState | `Active ⇄ Passive ⇄ Idle` | `GetLastInputInfo()` polled every 5s |
| DisplayState | `On ⇄ Off` | `RegisterPowerSettingNotification(GUID_MONITOR_POWER_ON)` |

**Screen-time rule:**

```rust
screen_time = system == Unlocked && activity != Idle && display == On
```

Time accounting:
- Active screen time = Unlocked + Active + On
- Passive screen time = Unlocked + Passive + On
- Total screen time = Active + Passive
- Idle/Locked/Sleep = their own buckets

## 3. Timestamps & Local Days

- **Storage**: UTC ISO-8601 strings (`2026-11-13T14:27:00Z`).
- **Local day**: every time-series table carries a precomputed `local_date TEXT NOT NULL`
  (`YYYY-MM-DD` in the local timezone). All dashboards queries filter by `local_date`, never
  `date(start_time)`.
- Midnight splitting uses the **local** midnight.

## 4. Crash Recovery & Heartbeat

- Heartbeat: single-row `heartbeat` table (`id = 1`), upsert every 30s and on every transition.
- Startup reconciliation: on boot, find sessions where `end_time IS NULL` or
  `status = 'active'`, close them at the last heartbeat timestamp, mark `status = 'recovered'`,
  and log.

## 5. IPC

- Pipe: `\\.\pipe\vigil_{USER_SID}`; JSON-RPC 2.0; newline-delimited; max 64KB frame.
- DACL restricted to the current user SID.
- Service is the sole SQLite writer; dashboard delegates mutations over IPC.

## 6. Performance Targets

- Service RAM ≤ 15 MB (pure Rust, no WebView embedded).
- Dashboard holds 0 MB when closed (process terminated).
- Profiling is measurement, not a promise — measure release builds on real workloads.

## 7. Privacy

- Data stays local by default; no cloud sync/telemetry without explicit consent.
- Browser: domain-level collection by default; excluded apps/domains honored; incognito windows
  are time-counted but not URL-recorded.
