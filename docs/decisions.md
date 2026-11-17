# Vigil — Architecture & Product Decisions

Record of decisions taken from the implementation plan and the design review
(`ScreenTime_PC_Review_Improvements_and_Open_Questions.docx`). Each entry resolves an open
question.

| # | Question | Decision |
|---|---|---|
| 1 | Audience | Personal utility → public product (phased). Privacy disclosure and signing added before distribution. |
| 2 | Browser collection default | **Domain-level by default**; full URL only with explicit opt-in. Incognito windows counted, not recorded. |
| 3 | IPC transport | **Windows Named Pipes** (`\\.\pipe\vigil_{USER_SID}`), JSON-RPC 2.0, DACL-restricted. |
| 4 | Database ownership | **Service is the sole writer**. Dashboard opens read-only or queries via IPC. |
| 5 | Screen-time definition | `system == Unlocked && activity != Idle && display == On`, with active/passive split via `active_ratio`. |
| 6 | Crash recovery | Single-row heartbeat + startup reconciliation → `status = 'recovered'`. Never drop or inflate time. |
| 7 | Idle/lock/sleep/monitor | Separate 3-dimension model with distinct measurement sources. |
| 8 | Browser tab granularity | Adapter interface per browser; best-effort active-window URL; no per-tab history in v1. |
| 9 | Incognito | Time counted, URL not recorded (explicit privacy policy). |
| 10 | Network | Interface-level deltas via `GetIfTable2`; counter-reset detection and clamping. No per-app attribution in v1. |
| 11 | Multi-user | Per-user data root under `%LOCALAPPDATA%\Vigil`; single-user service lifetime for v1. |
| 12 | Retention | Default 90 days with settings-driven purge; SQLite-aware backup via `VACUUM INTO`. |
| 13 | Week start | Start-of-week configurable, default Monday; local timezone for all presentation. |
| 14 | Min OS | Windows 11 build 22621+ (22H2). |
| 15 | Startup | Task Scheduler auto-start (opt-in at install), visible in tray; reliable uninstall path. |

## Build order (per review recommendation)

> **Build the monitoring engine first; build the dashboard second.**

1. Tracking prototype (Phase 1)
2. Data correctness (Phase 2)
3. Lifecycle + IPC (Phase 3)
4. Browser + network (Phase 4)
5. Dashboard (Phase 5)
6. Reports + release (Phase 6)
