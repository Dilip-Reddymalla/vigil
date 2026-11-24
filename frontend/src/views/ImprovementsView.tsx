import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchComparison, fetchWeekDetail, type Comparison, type Verdict, type WeekDetail } from "@/lib/api";
import { cn, formatDuration } from "@/lib/utils";
import { today } from "@/lib/date";

type Period = "week" | "month" | "week_detail";

/* ------------------------------------------------------------------ */
/* Verdict theming                                                    */
/* ------------------------------------------------------------------ */
const VERDICT: Record<
  Verdict,
  {
    label: string;
    sub: string;
    emoji: string;
    ring: string;
    glow: string;
    border: string;
    text: string;
  }
> = {
  good: {
    label: "A great week",
    sub: "You spent less time on screen and got more focused, deeper work done.",
    emoji: "✦",
    ring: "from-emerald-400/30 to-teal-500/10",
    glow: "radial-gradient(120% 120% at 10% 0%, rgba(16,185,129,0.22), transparent 60%)",
    border: "border-emerald-400/30",
    text: "text-emerald-600 dark:text-emerald-300",
  },
  mixed: {
    label: "A mixed week",
    sub: "Some things improved, others slipped. Small nudges add up.",
    emoji: "⚖",
    ring: "from-amber-400/30 to-orange-500/10",
    glow: "radial-gradient(120% 120% at 10% 0%, rgba(245,158,11,0.20), transparent 60%)",
    border: "border-amber-400/30",
    text: "text-amber-600 dark:text-amber-300",
  },
  needs_attention: {
    label: "Needs attention",
    sub: "Screen time went up while focus and deep work dipped. Time to reset.",
    emoji: "⚠",
    ring: "from-rose-500/30 to-red-500/10",
    glow: "radial-gradient(120% 120% at 10% 0%, rgba(244,63,94,0.22), transparent 60%)",
    border: "border-rose-400/30",
    text: "text-rose-500 dark:text-rose-300",
  },
};

/* ------------------------------------------------------------------ */
/* Metric specs                                                       */
/* ------------------------------------------------------------------ */
interface MetricSpec {
  key: string;
  label: string;
  accent: [string, string]; // gradient
  goodDown: boolean;
  fmt: (v: number | null) => string;
  pickCur: (c: Comparison) => number | null;
  pickPrev: (c: Comparison) => number | null;
  pickDelta: (c: Comparison) => number | null;
}

const METRICS: MetricSpec[] = [
  {
    key: "screen",
    label: "Screen time",
    accent: ["#0ea5e9", "#6366f1"],
    goodDown: true,
    fmt: (v) => formatDuration(v),
    pickCur: (c) => c.current.screen_ms,
    pickPrev: (c) => c.previous.screen_ms,
    pickDelta: (c) => c.deltas.screen_pct,
  },
  {
    key: "focus",
    label: "Focus score",
    accent: ["#10b981", "#059669"],
    goodDown: false,
    fmt: (v) => (v == null ? "—" : `${Math.round(v)}`),
    pickCur: (c) => c.current.focus,
    pickPrev: (c) => c.previous.focus,
    pickDelta: (c) => c.deltas.focus_pct,
  },
  {
    key: "deep",
    label: "Deep work",
    accent: ["#8b5cf6", "#6d28d9"],
    goodDown: false,
    fmt: (v) => formatDuration(v),
    pickCur: (c) => c.current.deep_ms,
    pickPrev: (c) => c.previous.deep_ms,
    pickDelta: (c) => c.deltas.deep_pct,
  },
  {
    key: "sessions",
    label: "Deep-work sessions",
    accent: ["#f59e0b", "#d97706"],
    goodDown: false,
    fmt: (v) => (v == null ? "—" : `${Math.round(v)}`),
    pickCur: (c) => c.current.deep_sessions,
    pickPrev: (c) => c.previous.deep_sessions,
    pickDelta: (c) => c.deltas.deep_sessions_pct,
  },
  {
    key: "distraction",
    label: "Distraction index",
    accent: ["#f43f5e", "#e11d48"],
    goodDown: true,
    fmt: (v) => (v == null ? "—" : v.toFixed(1)),
    pickCur: (c) => c.current.distraction,
    pickPrev: (c) => c.previous.distraction,
    pickDelta: (c) => c.deltas.distraction_pct,
  },
  {
    key: "active",
    label: "Active ratio",
    accent: ["#06b6d4", "#0891b2"],
    goodDown: false,
    fmt: (v) => (v == null ? "—" : `${Math.round(v * 100)}%`),
    pickCur: (c) => c.current.active_ratio,
    pickPrev: (c) => c.previous.active_ratio,
    pickDelta: (c) => c.deltas.active_pct,
  },
];

function DeltaBadge({ delta, goodDown }: { delta: number | null; goodDown: boolean }) {
  if (delta == null) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-(--surface-2) px-2 py-0.5 text-[11px] font-medium text-(--text-tertiary)">
        n/a
      </span>
    );
  }
  const good = goodDown ? delta < 0 : delta > 0;
  const sign = delta > 0 ? "↑" : delta < 0 ? "↓" : "→";
  const pct = Math.abs(delta).toFixed(0);
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-semibold tabular-nums",
        good
          ? "bg-emerald-500/12 text-emerald-600 dark:text-emerald-300"
          : delta === 0
            ? "bg-(--surface-2) text-(--text-tertiary)"
            : "bg-rose-500/12 text-rose-500 dark:text-rose-300",
      )}
    >
      {sign} {pct}%
    </span>
  );
}

function MetricValue({ m, c }: { m: MetricSpec; c: Comparison }) {
  const cur = m.pickCur(c);
  const prev = m.pickPrev(c);
  const delta = m.pickDelta(c);
  return (
    <div
      className="group relative overflow-hidden rounded-2xl border border-(--border-subdued) bg-(--surface) p-5 transition-all duration-300 hover:-translate-y-1 hover:shadow-xl"
      style={{ boxShadow: "var(--shadow-card)" }}
    >
      {/* top accent wash */}
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-1 opacity-80"
        style={{ background: `linear-gradient(90deg, ${m.accent[0]}, ${m.accent[1]})` }}
      />
      <div
        className="pointer-events-none absolute -right-10 -top-10 size-28 rounded-full blur-2xl opacity-20 transition-opacity duration-300 group-hover:opacity-40"
        style={{ background: `radial-gradient(circle, ${m.accent[0]}, transparent 70%)` }}
      />
      <div className="flex items-center justify-between">
        <p className="text-[11px] font-semibold uppercase tracking-wider text-(--text-tertiary)">
          {m.label}
        </p>
        <DeltaBadge delta={delta} goodDown={m.goodDown} />
      </div>
      <p className="mt-3 text-3xl font-bold tracking-tight tabular-nums">{m.fmt(cur)}</p>
      <p className="mt-1 text-xs text-(--text-tertiary)">
        last {m.fmt(prev)}
      </p>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Day-by-day chart                                                    */
/* ------------------------------------------------------------------ */
function DayChart({ c }: { c: Comparison }) {
  const [metric, setMetric] = useState<"screen" | "deep">("screen");
  const cur = c.daily.current;
  const prev = c.daily.previous;
  const days = Math.max(cur.length, prev.length, 7);

  const seriesCur = cur.map((d) => (metric === "screen" ? d.screen_ms : d.deep_ms));
  const seriesPrev = prev.map((d) => (metric === "screen" ? d.screen_ms : d.deep_ms));
  const max = Math.max(1, ...seriesCur, ...seriesPrev);

  const dayLabel = (i: number) => {
    const d = cur[i]?.date ?? prev[i]?.date;
    if (!d) return "";
    const dt = new Date(`${d}T00:00:00`);
    return dt.toLocaleDateString([], { weekday: "short" });
  };

  return (
    <div
      className="rounded-2xl border border-(--border-subdued) bg-(--surface) p-5"
      style={{ boxShadow: "var(--shadow-card)" }}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">Day by day</h3>
          <p className="text-xs text-(--text-tertiary)">Daily totals — this period vs previous</p>
        </div>
        <div className="flex rounded-full bg-(--surface-2) p-0.5 text-xs font-medium">
          {(["screen", "deep"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMetric(m)}
              className={cn(
                "rounded-full px-3 py-1 transition-colors",
                metric === m
                  ? "bg-(--surface) text-(--text-primary) shadow-sm"
                  : "text-(--text-tertiary) hover:text-(--text-secondary)",
              )}
            >
              {m === "screen" ? "Screen time" : "Deep work"}
            </button>
          ))}
        </div>
      </div>

      <div className="mt-5 flex items-end gap-[6px]">
        {Array.from({ length: days }).map((_, i) => {
          const cv = seriesCur[i] ?? 0;
          const pv = seriesPrev[i] ?? 0;
          const h = (v: number) => Math.max(4, Math.round((v / max) * 140));
          return (
            <div key={i} className="group flex flex-1 flex-col items-center gap-2">
              <div className="flex h-36 w-full items-end justify-center gap-1">
                <div
                  className="w-[38%] rounded-t-md transition-all duration-300 group-hover:opacity-100"
                  style={{
                    height: `${h(cv)}px`,
                    background: "linear-gradient(180deg, var(--accent), color-mix(in srgb, var(--accent) 55%, transparent))",
                    opacity: 0.92,
                    boxShadow: "0 0 0 1px inset rgba(255,255,255,0.04), 0 8px 18px -8px color-mix(in srgb, var(--accent) 45%, transparent)",
                  }}
                >
                  <div className="pointer-events-none -mt-8 hidden rounded-md bg-(--text-primary) px-2 py-1 text-[10px] font-medium text-(--background) shadow-lg group-hover:block">
                    {formatDuration(cv)} this
                  </div>
                </div>
                <div
                  className="w-[38%] rounded-t-md border border-(--border-subdued) transition-all duration-300 group-hover:opacity-100"
                  style={{
                    height: `${h(pv)}px`,
                    background: "linear-gradient(180deg, var(--accent-2), color-mix(in srgb, var(--accent-2) 40%, transparent))",
                    opacity: 0.55,
                  }}
                >
                  <div className="pointer-events-none -mt-8 hidden rounded-md bg-(--text-primary) px-2 py-1 text-[10px] font-medium text-(--background) shadow-lg group-hover:block">
                    {formatDuration(pv)} prev
                  </div>
                </div>
              </div>
              <span className="text-[10px] font-medium text-(--text-tertiary)">
                {dayLabel(i) || ""}
              </span>
            </div>
          );
        })}
      </div>

      <div className="mt-4 flex items-center gap-4 text-xs text-(--text-secondary)">
        <span className="inline-flex items-center gap-1.5">
          <i className="h-2.5 w-2.5 rounded-sm" style={{ background: "var(--accent)" }} /> this period
        </span>
        <span className="inline-flex items-center gap-1.5">
          <i className="h-2.5 w-2.5 rounded-sm" style={{ background: "var(--accent-2)", opacity: 0.55 }} /> previous
        </span>
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Main view                                                           */
/* ------------------------------------------------------------------ */
export function ImprovementsView() {
  const [period, setPeriod] = useState<Period>("week");
  const { data, isLoading } = useQuery({
    queryKey: ["comparison", period],
    queryFn: () => fetchComparison(period as "week" | "month"),
    enabled: period !== "week_detail",
    refetchInterval: 60_000,
  });

  const heroMetrics = METRICS.slice(0, 3);
  const gridMetrics = METRICS.slice(3);

  return (
    <div className="mx-auto max-w-5xl space-y-6 pb-12">
      {/* Header */}
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.18em] text-(--text-tertiary)">
            Personal analytics
          </p>
          <h1 className="text-2xl font-bold tracking-tight">
            Improvements
            <span className="ml-2 bg-gradient-to-r from-(--accent) to-(--accent-2) bg-clip-text font-bold text-transparent">
              this period
            </span>
          </h1>
        </div>
        <div className="flex rounded-full border border-(--border-subdued) bg-(--surface) p-1 shadow-sm">
          {([
            ["week", "Week"],
            ["month", "Month"],
            ["week_detail", "Pick week"],
          ] as [Period, string][]).map(([p, label]) => (
            <button
              key={p}
              type="button"
              onClick={() => setPeriod(p)}
              className={cn(
                "rounded-full px-4 py-1.5 text-sm font-medium transition-all duration-200",
                period === p
                  ? "bg-(--text-primary) text-(--background) shadow-md"
                  : "text-(--text-secondary) hover:text-(--text-primary)",
              )}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {period === "week_detail" ? (
        <WeekDetailPanel />
      ) : isLoading && !data ? (
        <div className="space-y-6">
          <div className="h-32 animate-pulse rounded-2xl bg-(--surface-2)" />
          <div className="grid gap-4 sm:grid-cols-3">
            {[0, 1, 2].map((i) => (
              <div key={i} className="h-36 animate-pulse rounded-2xl bg-(--surface-2)" />
            ))}
          </div>
        </div>
      ) : !data ? (
        <div className="rounded-2xl border border-dashed border-(--border-subdued) p-12 text-center text-sm text-(--text-tertiary)">
          Not enough data yet — keep using Vigil and come back to see your trend.
        </div>
      ) : (
        <>
          {/* Verdict banner */}
          <VerdictBanner data={data} />

          {/* Hero metrics */}
          <div className="grid gap-4 sm:grid-cols-3">
            {heroMetrics.map((m) => (
              <MetricValue key={m.key} m={m} c={data} />
            ))}
          </div>

          {/* Secondary metrics */}
          <div className="grid gap-4 sm:grid-cols-3">
            {gridMetrics.map((m) => (
              <MetricValue key={m.key} m={m} c={data} />
            ))}
          </div>

          {/* Day chart (week only) */}
          {data.period === "week" && <DayChart c={data} />}
        </>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Pick-a-week panel                                                    */
/* ------------------------------------------------------------------ */
function WeekMetricCard({ label, value, accent }: { label: string; value: string; accent: [string, string] }) {
  return (
    <div
      className="group relative overflow-hidden rounded-2xl border border-(--border-subdued) bg-(--surface) p-5 transition-all duration-300 hover:-translate-y-1 hover:shadow-xl"
      style={{ boxShadow: "var(--shadow-card)" }}
    >
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-1 opacity-80"
        style={{ background: `linear-gradient(90deg, ${accent[0]}, ${accent[1]})` }}
      />
      <div
        className="pointer-events-none absolute -right-10 -top-10 size-28 rounded-full blur-2xl opacity-20 transition-opacity duration-300 group-hover:opacity-40"
        style={{ background: `radial-gradient(circle, ${accent[0]}, transparent 70%)` }}
      />
      <p className="text-[11px] font-semibold uppercase tracking-wider text-(--text-tertiary)">{label}</p>
      <p className="mt-3 text-3xl font-bold tracking-tight tabular-nums">{value}</p>
    </div>
  );
}

function WeekDayChart({ data }: { data: WeekDetail }) {
  const [metric, setMetric] = useState<"screen" | "deep" | "focus">("screen");
  const days = data.daily;
  const series = days.map((d) =>
    metric === "screen" ? d.screen_ms : metric === "deep" ? d.deep_ms : (d.focus ?? 0),
  );
  const max = Math.max(1, ...series);
  const fmt = (v: number) => (metric === "focus" ? `${Math.round(v)}` : formatDuration(v));

  return (
    <div className="rounded-2xl border border-(--border-subdued) bg-(--surface) p-5" style={{ boxShadow: "var(--shadow-card)" }}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">Day by day</h3>
          <p className="text-xs text-(--text-tertiary)">Daily values within this week</p>
        </div>
        <div className="flex rounded-full bg-(--surface-2) p-0.5 text-xs font-medium">
          {(["screen", "deep", "focus"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMetric(m)}
              className={cn(
                "rounded-full px-3 py-1 transition-colors",
                metric === m
                  ? "bg-(--surface) text-(--text-primary) shadow-sm"
                  : "text-(--text-tertiary) hover:text-(--text-secondary)",
              )}
            >
              {m === "screen" ? "Screen" : m === "deep" ? "Deep" : "Focus"}
            </button>
          ))}
        </div>
      </div>

      <div className="mt-5 flex items-end gap-[6px]">
        {days.map((d, i) => {
          const v = series[i] ?? 0;
          const h = Math.max(4, Math.round((v / max) * 150));
          return (
            <div key={d.date} className="group flex flex-1 flex-col items-center gap-2">
              <div className="flex h-36 w-full items-end justify-center">
                <div
                  className="w-full rounded-t-md transition-all duration-300 group-hover:opacity-100"
                  style={{
                    height: `${h}px`,
                    background: "linear-gradient(180deg, var(--accent), color-mix(in srgb, var(--accent) 55%, transparent))",
                    opacity: 0.92,
                    boxShadow: "0 8px 18px -8px color-mix(in srgb, var(--accent) 45%, transparent)",
                  }}
                >
                  <div className="pointer-events-none -mt-8 hidden rounded-md bg-(--text-primary) px-2 py-1 text-[10px] font-medium text-(--background) shadow-lg group-hover:block">
                    {d.date} · {fmt(v)}
                  </div>
                </div>
              </div>
              <span className="text-[10px] font-medium text-(--text-tertiary)">
                {new Date(`${d.date}T00:00:00`).toLocaleDateString([], { weekday: "short" })}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function WeekDetailPanel() {
  const [anchor, setAnchor] = useState(today());
  const { data, isLoading } = useQuery({
    queryKey: ["week_detail", anchor],
    queryFn: () => fetchWeekDetail(anchor),
    enabled: !!anchor,
  });

  const cards = data
    ? [
        { label: "Screen time", value: formatDuration(data.metrics.screen_ms), accent: ["#0ea5e9", "#6366f1"] as [string, string] },
        { label: "Focus score", value: data.metrics.focus == null ? "—" : `${Math.round(data.metrics.focus)}`, accent: ["#10b981", "#059669"] as [string, string] },
        { label: "Deep work", value: formatDuration(data.metrics.deep_ms), accent: ["#8b5cf6", "#6d28d9"] as [string, string] },
        { label: "Deep sessions", value: `${data.metrics.deep_sessions}`, accent: ["#f59e0b", "#d97706"] as [string, string] },
        { label: "Distraction", value: data.metrics.distraction == null ? "—" : data.metrics.distraction.toFixed(1), accent: ["#f43f5e", "#e11d48"] as [string, string] },
        { label: "Active ratio", value: data.metrics.active_ratio == null ? "—" : `${Math.round(data.metrics.active_ratio * 100)}%`, accent: ["#06b6d4", "#0891b2"] as [string, string] },
      ]
    : [];

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-end gap-4">
        <label className="space-y-1.5">
          <span className="text-xs font-medium text-(--text-secondary)">Pick a date in the week (Mon–Sun)</span>
          <input
            type="date"
            value={anchor}
            onChange={(e) => e.target.value && setAnchor(e.target.value)}
            className="block rounded-xl border border-(--border-subdued) bg-(--surface) px-4 py-2 text-sm shadow-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
          />
        </label>
        {data && (
          <p className="text-xs text-(--text-tertiary)">
            Week of {data.ranges.current.start} → {data.ranges.current.end}
          </p>
        )}
      </div>

      {isLoading && !data ? (
        <div className="grid gap-4 sm:grid-cols-3">
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="h-32 animate-pulse rounded-2xl bg-(--surface-2)" />
          ))}
        </div>
      ) : !data || cards.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-(--border-subdued) p-12 text-center text-sm text-(--text-tertiary)">
          No data for that week yet.
        </div>
      ) : (
        <>
          <div className="grid gap-4 sm:grid-cols-3">
            {cards.map((c) => (
              <WeekMetricCard key={c.label} label={c.label} value={c.value} accent={c.accent} />
            ))}
          </div>
          <WeekDayChart data={data} />
        </>
      )}
    </div>
  );
}

function VerdictBanner({ data }: { data: Comparison }) {
  const v = VERDICT[data.verdict] ?? VERDICT.mixed;
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-2xl border p-6",
        v.border,
        data.verdict === "good"
          ? "bg-gradient-to-br from-emerald-500/10 via-transparent to-teal-500/5"
          : data.verdict === "needs_attention"
            ? "bg-gradient-to-br from-rose-500/10 via-transparent to-red-500/5"
            : "bg-gradient-to-br from-amber-500/10 via-transparent to-orange-500/5",
      )}
      style={{ backgroundImage: v.glow }}
    >
      <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
        <div
          className={cn(
            "grid size-14 shrink-0 place-items-center rounded-2xl bg-gradient-to-br text-2xl text-white shadow-lg",
            v.ring,
          )}
        >
          {v.emoji}
        </div>
        <div className="min-w-0 flex-1">
          <h2 className={cn("text-2xl font-bold tracking-tight", v.text)}>{v.label}</h2>
          <p className="mt-0.5 text-sm text-(--text-secondary)">{v.sub}</p>
        </div>
      </div>

      {data.reasons.length > 0 && (
        <div className="mt-5 flex flex-wrap gap-2">
          {data.reasons.map((r, i) => (
            <span
              key={i}
              className={cn(
                "inline-flex items-center rounded-full border bg-(--surface)/70 px-3 py-1 text-xs font-medium backdrop-blur",
                r.endsWith("✓")
                  ? "border-emerald-500/20 text-emerald-600 dark:text-emerald-300"
                  : "border-rose-500/20 text-rose-500 dark:text-rose-300",
              )}
            >
              {r}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
