# Vigil — Implementation Plan (referenced copy)

> Source of truth: `c:\Users\DILIP REDDYMALLA\.gemini\antigravity\brain\afe2ab6d-0795-45a3-81dc-97d73b224da6\implementation-plan.md` (v2.1).

The full plan lives at the path above. This repository implements it. A summary of
the architecture, phases, and decisions is maintained in:

- [`architecture.md`](./architecture.md) — process split, state dimensions, timestamps,
  recovery, IPC, performance, privacy.
- [`decisions.md`](./decisions.md) — resolution of the design-review open questions.
- [`README.md`](../README.md) — layout and phase status.

## Phases

| Phase | Scope | Repository location |
|---|---|---|
| 1 | Tracking prototype (window events, input, lock/sleep/monitor, state machine) | `src-service/` |
| 2 | Data correctness: SQLite schema, migrations, heartbeat, crash recovery | `src-service/src/db` |
| 3 | Lifecycle & IPC: Named Pipe server, tray, Task Scheduler | `src-service/src/ipc` |
| 4 | Browser (UI Automation) & network (`GetIfTable2`) modules | `src-service/src/trackers` |
| 5 | Tauri v2 dashboard UI | `src-tauri/`, `frontend/` |
| 6 | Reports & release (PDF/CSV, NSIS installer, profiling) | repo-wide |

## Phase 1 exit condition

> Service outputs accurate, non-overlapping transition logs.

Verified by inspecting the JSON transition stream: a full-day log matches observed
app usage and idle periods; no overlapping or double-counted sessions.
