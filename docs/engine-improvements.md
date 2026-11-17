# Vigil Engine Improvements — Implementation & Feasibility

> Source: `engine-improvement-plan.md` (v1.0). Record of which improvements are
> **implemented**, and a feasibility assessment for the remainder.

## ✅ Implemented (in `vigil-service`)

| Phase | Feature | Status |
|---|---|---|
| 1 | Fullscreen shield (`SHQueryUserNotificationState` → clamp Idle→Passive) | ✅ |
| 1 | NULL-HWND D3D recovery (don't drop session in immersive state) | ✅ |
| 1 | Active-audio shield (WASAPI `IAudioMeterInformation` peak > 0.001) | ✅ |
| 2 | XInput controller polling (packet-change = Active) | ✅ |
| 2 | Anti-cheat process fallback (`CreateToolhelp32Snapshot` when `OpenProcess` denied) | ✅ |
| 3 | Sub-second session collapsing (< 2s, same process → merge into prior) | ✅ |
| 3 | Media title normalization (VLC / MPC / PotPlayer / mpv) | ✅ |
| 4 | Presentation-mode awareness (included in fullscreen clamp) | ✅ |
| 5 | Domain-level category intelligence (`domain_categories`, `productive_browser_ms`) | ✅ |
| 6.1 | Project / workspace context (`project_name` column, IDE title parser) | ✅ |
| 7.1 | Daily app/site limits backend (`usage_limits` setting, `limit_nudge` events) | ✅ |
| 7.2 | Deep-work / flow detection (45 min unbroken, `focus_streak_days`) | ✅ |
| 8 | Multi-monitor attribution (`monitor_index`, `EnumDisplayMonitors`) | ✅ |
| 9 | Battery / energy impact (`battery_snapshots`, `GetSystemPowerStatus`) | ✅ |
| 11.1 | Obsidian / Markdown daily note export (`reports.export_md`) | ✅ |
| 10 | WAL checkpoint + `TRUNCATE` + working-set trimming in housekeeping | ✅ |

## 🔍 Feasibility assessment (Phase 5–11)

| Phase | Feature | Complexity | Feasible? | Notes |
|---|---|---|---|---|
| 5 | Domain-level category intelligence | Small | ✅ Yes | Map `normalized_domain` → category for productivity score; we already store domain + `is_browser`. |
| 6.1 | IDE workspace/segment resolver (`project_name`) | Medium | ✅ Yes | Parse VS Code/CLion titles into project; needs a new `project_name` column + view. |
| 6.2 | Document/PDF title resolver | Small | ✅ Yes | Strip ` - Word`/` - Adobe Acrobat` suffixes (like media filename). |
| 7.1 | Daily app/site limits + toasts | Medium | ✅ Yes | Add allowance settings + `service` timer + Windows toast (needs a toast crate/API). |
| 7.2 | Deep-work / flow detection (45min unbroken) | Medium | ✅ Yes | We already count deep_work sessions; extend to require no social/chat switch. |
| 7.3 | Switching-rate metric | Small | ✅ Done | `distraction_index` (app switches/hour) already computed. |
| 7.4 | First-pickup & night-owl sleep hygiene | Small | ✅ Yes | Read first unlock + late-night (23:00–05:00) totals per day. |
| 8 | Multi-monitor / dual-display attribution | Medium | ✅ Yes | `EnumDisplayMonitors` + `MonitorFromWindow`; store monitor index. |
| 9 | Battery / energy impact | Medium | ✅ Yes | `GetSystemPowerStatus` polling on battery; correlate drain with foreground app. |
| 10 | WAL checkpoint + memory trim | Small | ✅ Done | Implemented in housekeeping. |
| 11.1 | Obsidian / Markdown daily note export | Medium | ✅ Yes | Reuse CSV/PDF report path to also emit a `.md` summary. |
| 11.2 | Weekly delta & trends cards | Medium | ✅ Yes | Compare `daily_summaries` week-over-week. |

**Bottom line:** All remaining phases (5–11) are **implementable** with the current
architecture. Complexity ranges from small (5, 6.2, 7.4, 11.1) to medium
(6.1, 7.1, 7.2, 8, 9, 11.2). Items 7.3 and 10 are already done.

## Suggested next priorities
1. **Domain-level category intelligence (5)** — improves productivity-score accuracy.
2. **Deep-work / flow detection (7.2)** — already partially there; extend the rule.
3. **Obsidian/Markdown export (11.1)** — reuses the report pipeline.
4. **App/site daily limits + toasts (7.1)** — the most user-visible "digital wellbeing" feature.
5. **Multi-monitor attribution (8)** — valuable for dual-display users.
