# Improvements Page — Design Plan

A new **sidebar tab: "Improvements"** that answers *"was this period better than the last?"*
for screen time, focus, and deep work. Compares the **current period vs the previous period**
in two modes (tabs): **Week** and **Month**.

---

## 1. Period definitions

| Tab | Current period | Previous period |
| --- | --- | --- |
| **Week** | Monday → Sunday containing today | The prior Mon → Sun (7 days) |
| **Month** | 1st → last day of current month | 1st → last day of previous month |

> Week = Monday start. Month = calendar month. For month comparison we show **totals +
> per-day averages** (months have different lengths, so raw totals alone can mislead).

---

## 2. Data model (backend)

New RPC: `overview.compare` with `{ period: "week" | "month" }`.

Returns:

```
{
  period: "week" | "month",
  current: { screen_ms, focus_score, deep_ms, deep_sessions, distraction, active_ratio, days },
  previous: { same shape },
  deltas: { screen_pct, focus_pct, deep_pct, deep_sessions_pct, distraction_pct, active_pct },
  verdict: "good" | "mixed" | "needs_attention",
  reasons: [ "screen ↓ 12% ✓", "deep work ↑ 40% ✓", "distraction ↑ 9% ✗", ... ],
  daily: {                       // only for week tab (7 points each)
    current: [ {date, screen_ms, deep_ms, focus} ... ],
    previous: [ ... ]
  }
}
```

- Aggregated from `daily_summaries` (already has screen time, focus score, deep work,
  distraction, active ratio). No new tracking.
- Deltas are percentages; for month we also expose per-day averages.
- Verdict logic (below) is a pure function → unit-testable.

---

## 3. Verdict logic

Each metric contributes a `+1` if it moved the "good" way:

| Metric | Good = |
| --- | --- |
| Screen time | decreased |
| Focus score | increased |
| Deep work time | increased |
| Deep work sessions | increased |
| Distraction index | decreased |
| Active ratio | increased |

`favorable = number of +1`.

**Verdict:**
- `needs_attention` if screen time increased **and** (focus decreased **or** deep work decreased).
- else `good` if `favorable >= 4`.
- else `mixed`.

**Reasons** are the per-metric lines with ✓/✗ and the % change.

---

## 4. Component tree

```
ImprovementsView
├── PageHeader (title + subtitle)
├── PeriodTabs          // segmented "Week | Month"
├── VerdictBanner       // headline outcome
├── MetricStrip         // 3 headline cards (screen, focus, deep work)
├── MetricGrid          // remaining metrics (distraction, active ratio, sessions)
│   └── MetricCard
└── DayComparisonChart  // grouped bars current vs previous (week tab)
    └── Legend
```

**Components:**

1. `PeriodTabs` — pill toggle (Week/Month). Switches all data below.
2. `VerdictBanner` — colored banner + icon + 1-line summary + reasons chips.
3. `MetricStrip` — 3 compact cards; each = label, current value big, delta badge (↑/↓ %).
4. `MetricCard` — used in grid: label, **current** value, **previous** value, delta badge, colored.
5. `DayComparisonChart` — grouped bar chart, 2 series per day (current vs previous), with legend. Reuses the app's bar styling patterns.
6. `DeltaBadge` — small pill: `↑ 12%` (green if good, red if bad, gray if neutral).

---

## 5. Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Improvements                                     [ Week | Month ] │
├──────────────────────────────────────────────────────────────┤
│ ┌──────────────────────────────────────────────────────────┐ │
│ │  ✅ Good week                          (verdict banner)   │ │
│ │  Screen time ↓ 12% · Focus ↑ 8% · Deep work ↑ 40% ...     │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐                │
│ │ Screen time │ │ Focus score │ │ Deep work  │   (strip 3)   │
│ │ 31h 40m    │ │ 68/100      │ │ 6h 15m    │                │
│ │ ↓ 12%  ✓   │ │ ↑ 8%   ✓    │ │ ↑ 40%  ✓   │                │
│ └────────────┘ └────────────┘ └────────────┘                │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐                │
│ │ Distraction │ │ Active ratio│ │ Sessions    │   (grid)      │
│ │ 22 /h → 20 │ │ 74% → 78%  │ │ 110 → 124   │                │
│ └────────────┘ └────────────┘ └────────────┘                │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │  Day-by-day: current vs previous        (bars, week tab)  │ │
│ │  ▓ this week   ░ last week                                │ │
│ │  M  T  W  T  F  S  S                                     │ │
│ └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

- Max width container (`max-w-5xl` / `max-w-6xl`), consistent with other views.
- Month tab swaps the day chart for a small **per-day-average comparison** row.

---

## 6. Colors

Reuse the app's existing tokens (no new palette).

| Role | Token / color | Used for |
| --- | --- | --- |
| Positive / good | `--cat-productivity` → `#10b981` (emerald) | ↑ good deltas, verdict-banner glow |
| Negative / bad | `--cat-social_media` → `#f43f5e` (rose) | ↓ bad deltas, warning text |
| Neutral | `--cat-system` → `#64748b` (slate) | unchanged metrics |
| Accent (primary) | `var(--accent)` | interactive elements, chart series A |
| Accent-2 | `var(--accent-2)` | chart series B (last week) |
| Surfaces | `--surface`, `--surface-2`, `--border-subdued` | cards |
| Text | `--text-primary / secondary / tertiary` | hierarchy |

**Verdict banner color logic:**
- `good` → emerald-tinted card + emerald left border + ✅ icon.
- `mixed` → amber (`--cat-entertainment`) tint + ⚖️ icon.
- `needs_attention` → rose tint + ⚠️ icon.

**Delta badge:**
- green chevron ↑ (good), red chevron ↓ (bad), gray for `mixed`/neutral signs.
- Sign flips per metric definition (e.g. screen-time ↓ is green).

### Chart styling
- Bars: current week = `var(--accent)`, last week = `var(--accent-2)` with `opacity-70`.
- Hover tooltip mirrors NetworkView (dark pill with values).
- Y-axis implicit (scaled to max), like the rest of the app.

---

## 7. States

- **Loading:** skeleton cards / pulse (same pattern as Dashboard).
- **Empty data:** if the previous period has no summaries, show a friendly empty state
  ("Not enough history yet — keep using Vigil and come back.").
- **Current week incomplete:** still compare with what's recorded; note "week in progress".

---

## 8. Backend work (short)

- `db/trends.rs` (new): `compare_periods(conn, period)` using `resolve_range` from `report.rs`.
- Add `overview.compare` to `rpc.rs` + `engine` method.
- Unit tests: verdict logic + week/month range boundaries + delta calcs.

---

## 9. Frontend work (short)

- `views/ImprovementsView.tsx` + `components/period/{PeriodTabs, VerdictBanner, MetricStrip, MetricCard, DeltaBadge, DayComparisonChart}.tsx`.
- Add tab to `Sidebar.tsx` + route in `App.tsx`.
- `lib/api.ts`: `fetchComparison(period)`.
- Reuse existing `Card`, `Icon`, `formatDuration`, `formatBytes`.

---

## 10. Deliverables order (1. backend → 2. frontend → 3. build/package)
1. Implement `db/trends.rs` + RPC + tests (`cargo test` fast).
2. Implement frontend components + tab + route (`npm run build`).
3. Rebuild dashboard release, refresh dist/installer/zip, bump release → `v0.1.4` and push.
