# Vigil Feature Plan — Goals, Implementation & Expected Impact

> Priority-ordered plan for the most important remaining features. Each entry
> states the **end goal**, the **implementation approach**, and the **expected
> performance / improvement**.
>
> **Status:** Items 1–7 are **implemented** in `vigil-service` (backend + tested),
> with the frontend built. Item 8 (WAL/memory extension) is partially in place.
>
> ```text
> 1. Domain category intelligence   ✅ (domain_categories, productive_browser_ms)
> 2. Deep-work / flow detection     ✅ (45 min threshold, focus_streak_days)
> 3. Daily app/site limits           ✅ (usage_limits setting, limit_nudge events)
> 4. Project context                 ✅ (project_name column, IDE title parser)
> 5. Multi-monitor attribution       ✅ (monitor_index, EnumDisplayMonitors)
> 6. Battery / energy impact         ✅ (battery_snapshots, GetSystemPowerStatus)
> 7. Obsidian / Markdown export      ✅ (reports.export_md, report_md)
> 8. Storage & memory guarantees     🟡 (WAL trim done; 30-min timer TBD)
> ```

---

## 1. Domain-Level Category Intelligence (Phase 5)

- **End goal:** Productivity, focus, and distraction metrics are computed from the
  *actual domains* the user visits (e.g. GitHub → development, Kick/YouTube →
  entertainment), not from blanket browser time.
- **Implementation:**
  1. Add a `domain → category` mapping table (seeded + user-editable in Settings).
  2. When recording a browser visit (`record_visit`), derive the category from the
     domain and store it (or compute on read from `browser_visits.normalized_domain`).
  3. `daily_summaries.productivity_score` uses domain-weighted time instead of the
     browser's coarse `is_browser` category.
  4. Surface a "Top sites by category" breakdown on the Dashboard.
- **Expected impact:** Accuracy of the focus score (currently ~99% due to browser
  time counting as productive) moves to a truthful value — e.g. a YouTube-heavy day
  correctly scores low ~ a terminal/GitHub day high. Low CPU cost (one lookup per
  visit).

---

## 2. Deep-Work / Flow-State Detection (Phase 7.2)

- **End goal:** Detect unbroken 45+ min stretches in `development`/`productivity`
  with zero switches to social/messaging; report sessions, time, and weekly streaks.
- **Implementation:**
  1. In the engine, track a "deep-work streak" per session: reset the timer whenever a
     session's category is `social_media`/`communication` or an app switch occurs.
  2. When a `development`/`productivity` session reaches 45 min uninterrupted, mark
     `is_deep_work = 1` and accumulate `deep_work_time_ms`.
  3. Add an aggregation to `daily_summaries` (already has the columns) + a weekly
     streak table.
- **Expected impact:** Meaningful focus analytics (vs. today's always-zero deep-work
  figure on some days). Negligible cost — state tracked in-memory.

---

## 3. Daily App / Website Limits + Native Toasts (Phase 7.1)

- **End goal:** Users set daily allowances (e.g. `youtube.com`: 120 min); Vigil
  shows a Windows 11 toast at 90% and 100% and can soft-block.
- **Implementation:**
  1. Settings UI to define limits (app process or domain → minutes).
  2. Service tally: accumulate per-app/domain time for today (from sessions/visits).
  3. A 60s timer checks tallies; on crossing 90%/100%, dispatch a toast via the
     `windows` crate (or `winrt`). Mark `goal_met`/limit state.
  4. Optional "soft nudge" overlay in the dashboard.
- **Expected impact:** Core "digital wellbeing" enforcement; distinct product value.
  Cost: one SQL sum per minute (cheap) + toast API call only on threshold crossing.

---

## 4. Project & Workspace Context (Phase 6.1)

- **End goal:** Reports group time by *project/repo* (e.g. "screentimepc: 3h"),
  not just "VS Code".
- **Implementation:**
  1. Add `project_name TEXT` column to `app_sessions` (migration v3).
  2. `trackers/media`-style parser for IDE titles: `"engine.rs — screentimepc — Visual Studio Code"`
     → project `screentimepc`, file `engine.rs`.
  3. New **Projects** dashboard view aggregating by `project_name`.
- **Expected impact:** A "what did I actually work on" dimension; high value for
  developers. Cost: title-parsing only (minimal).

---

## 5. Multi-Monitor Attribution (Phase 8)

- **End goal:** Credit each session to the physical monitor that hosts the active
  window; support "docs on monitor 2 while typing on monitor 1".
- **Implementation:**
  1. On window change, call `MonitorFromWindow(hwnd)` + `GetMonitorInfoW` to get the
     monitor index; store on the session.
  2. Add a monitor filter/breakdown in the Dashboard.
- **Expected impact:** Dual-display users get correct per-monitor totals. Cheap —
  one Win32 call per foreground change.

---

## 6. Battery / Energy Impact (Phase 9)

- **End goal:** Show per-app energy drain (% battery/hour) on laptops.
- **Implementation:**
  1. Poll `GetSystemPowerStatus` every 60s while on battery.
  2. Track foreground process durations; compute drain rate and attribute share.
  3. New **Energy** tab.
- **Expected impact:** Identifies battery-hogging apps (streaming/compile). Cost:
  one cheap system call per minute.

---

## 7. Obsidian / Markdown Daily Note Export (Phase 11.1)

- **End goal:** One-click append of a clean Markdown daily summary into the user's
  Obsidian vault/journal.
- **Implementation:**
  1. Extend the report pipeline to render a `.md` summary (top apps, sites, screen
     time, deep work) — reuse `build_report`.
  2. Add a `reports.export_md` RPC + a Reports button; path from Settings.
- **Expected impact:** PKM power-users get automatic daily logs. Negligible cost.

---

## 8. Storage & Memory Guarantees (Phase 10) — *done, to extend*

- **End goal:** `data.db-wal` < 1 MB; service RAM ≤ 5 MB when idle.
- **Implementation (in place):** WAL checkpoint `TRUNCATE` + `SetProcessWorkingSetSizeEx`
  in housekeeping.
- **Extension:** Run checkpoint on a 30-min timer, and memory-trim on lock/idle
  transitions (not only housekeeping).
- **Expected impact:** Predictable disk growth and a smaller resident set.

---

## Suggested execution order (value ÷ effort)

| Priority | Feature | Effort | Value |
|---|---|---|---|
| 1 | Domain category intelligence | Small | High |
| 2 | Deep-work/flow detection | Medium | High |
| 3 | App/site limits + toasts | Medium | High |
| 4 | Project context | Medium | High |
| 5 | Obsidian/Markdown export | Small | Medium |
| 6 | Multi-monitor attribution | Medium | Medium |
| 7 | Battery/energy impact | Medium | Medium |
| 8 | WAL/memory extension | Small | Medium |

**Cross-cutting principles:** all work stays in-app (no extensions), local-only,
and within the service's low-CPU/RAM budget (no continuous polling beyond the
existing 5s/30s/60s cadences).
