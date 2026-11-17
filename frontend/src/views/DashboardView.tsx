import { useQuery } from "@tanstack/react-query";
import { useDashboard } from "@/hooks/useDashboard";
import { useDataLogger } from "@/hooks/useDataLogger";
import { fetchApps, fetchSites, getSystemName } from "@/lib/api";
import { formatDuration, formatBytes, categoryColor } from "@/lib/utils";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { ActivityRing } from "@/components/charts/ActivityRing";
import { DonutChart } from "@/components/charts/DonutChart";
import { HourlyTimeline, type HourBucket } from "@/components/charts/HourlyTimeline";
import { EMPTY_SUMMARY, type DashboardSummary } from "@/types";
import { useSelectedDate } from "@/lib/date";

const CATEGORY_HEX: Record<string, string> = {
  productivity: "#10b981",
  development: "#2563eb",
  communication: "#7c3aed",
  entertainment: "#f59e0b",
  social_media: "#f43f5e",
  browsing: "#0284c7",
  system: "#64748b",
  other: "#9aa1ab",
};

const DEFAULT_GOAL_MS = 8 * 3_600_000;

function statTiles(s: DashboardSummary) {
  return [
    { label: "Active", value: formatDuration(s.activeTimeMs), color: "var(--cat-productivity)" },
    { label: "Passive", value: formatDuration(s.passiveTimeMs), color: "var(--cat-browsing)" },
    { label: "Network ↑", value: formatBytes(s.networkSentBytes), color: "var(--accent)" },
    { label: "Network ↓", value: formatBytes(s.networkRecvBytes), color: "var(--accent-2)" },
    { label: "Idle", value: formatDuration(s.idleTimeMs), color: "var(--cat-entertainment)" },
    { label: "Sleep", value: formatDuration(s.sleepTimeMs), color: "var(--cat-communication)" },
  ];
}

export function DashboardView() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data, isLoading } = useDashboard(date, refreshEpoch);
  const s: DashboardSummary = data ?? EMPTY_SUMMARY;
  useDataLogger("Dashboard.summary", s, { isLoading });

  const { data: displayName } = useQuery({
    queryKey: ["system-name"],
    queryFn: () => getSystemName(),
    refetchInterval: 30_000,
  });

  const { data: apps = [] } = useQuery({
    queryKey: ["apps", date, refreshEpoch],
    queryFn: () => fetchApps(date),
    refetchInterval: 5_000,
  });
  useDataLogger("Dashboard.apps", apps);
  const { data: sites = [] } = useQuery({
    queryKey: ["sites", date, refreshEpoch],
    queryFn: () => fetchSites(date),
    refetchInterval: 5_000,
  });
  useDataLogger("Dashboard.sites", sites);
  const categoryMap = new Map<string, number>();
  for (const a of apps) {
    categoryMap.set(a.category, (categoryMap.get(a.category) ?? 0) + a.total_ms);
  }
  const pieData = [...categoryMap.entries()]
    .map(([category, value]) => ({ name: category, value }))
    .sort((a, b) => b.value - a.value);

  const slices = pieData.map((e) => ({
    label: e.name,
    value: e.value,
    color: CATEGORY_HEX[e.name] ?? "#9aa1ab",
  }));

  const goalMs = s.goalMs || DEFAULT_GOAL_MS;
  const screenRatio = Math.min(1, s.totalScreenTimeMs / goalMs);
  const deepRatio = s.totalScreenTimeMs > 0 ? s.deepWorkTimeMs / s.totalScreenTimeMs : 0;
  const activeRatio = s.totalScreenTimeMs > 0 ? s.activeTimeMs / s.totalScreenTimeMs : 0;

  const hourly: number[] = Array.isArray(s.hourlyBreakdownMs) ? s.hourlyBreakdownMs : Array(24).fill(0);
  const buckets: HourBucket[] = hourly.map((ms, h) => ({
    label: String(h).padStart(2, "0"),
    ms,
  }));

  return (
    <div className="mx-auto max-w-6xl space-y-5 pb-10">
      {/* Hero */}
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-sm font-medium text-(--text-secondary)">Overview</p>
          <h1 className="text-2xl font-semibold tracking-tight text-balance">
            Good day{displayName ? `, ${displayName}` : ""}
          </h1>
        </div>
        <div className="flex items-center gap-3">
          <div className="rounded-full bg-(--surface) px-4 py-1.5 text-xs font-medium text-(--text-secondary) ring-1 ring-(--border-subdued)">
            <span className="mr-2 inline-block size-1.5 rounded-full bg-(--cat-productivity) align-middle" />
            Service running
          </div>
          <div className="rounded-full bg-(--surface) px-4 py-1.5 text-xs font-medium text-(--text-primary) ring-1 ring-(--border-subdued)">
            Goal {formatDuration(goalMs)}
          </div>
        </div>
      </div>

      {isLoading ? (
        <div className="card flex h-48 items-center justify-center text-sm text-(--text-secondary)">Loading…</div>
      ) : (
        <>
          {/* Rings + quick stats */}
          <div className="grid gap-5 lg:grid-cols-12">
            <Card className="lg:col-span-5">
              <CardHeader>
                <div>
                  <CardTitle>Screen time</CardTitle>
                  <CardDescription>vs your daily goal</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="flex flex-col items-center gap-4 pt-4">
                <ActivityRing
                  size={210}
                  segments={[
                    { value: screenRatio, color: "var(--accent)", label: "Screen time" },
                    { value: deepRatio, color: "var(--cat-development)", label: "Deep work" },
                    { value: activeRatio, color: "var(--cat-productivity)", label: "Active ratio" },
                  ]}
                >
                  <span className="text-3xl font-semibold tracking-tight">{formatDuration(s.totalScreenTimeMs)}</span>
                  <span className="text-xs text-(--text-secondary)">of {formatDuration(goalMs)}</span>
                </ActivityRing>
                <div className="flex flex-wrap justify-center gap-x-5 gap-y-1.5 text-[11px] text-(--text-secondary)">
                  <span className="inline-flex items-center gap-1.5"><i className="size-2 rounded-full bg-(--accent)" /> Screen time</span>
                  <span className="inline-flex items-center gap-1.5"><i className="size-2 rounded-full bg-(--cat-development)" /> Deep work</span>
                  <span className="inline-flex items-center gap-1.5"><i className="size-2 rounded-full bg-(--cat-productivity)" /> Active ratio</span>
                </div>
              </CardContent>
            </Card>

            <div className="grid gap-5 sm:grid-cols-2 lg:col-span-7 lg:grid-cols-3">
              {statTiles(s).map((t) => (
                <Card key={t.label}>
                  <CardContent className="px-5 py-4">
                    <p className="text-[11px] font-medium uppercase tracking-wide text-(--text-tertiary)">{t.label}</p>
                    <p className="mt-1.5 text-xl font-semibold tracking-tight" style={{ color: t.color }}>
                      {t.value}
                    </p>
                  </CardContent>
                </Card>
              ))}
              <Card>
                <CardContent className="px-5 py-4">
                  <p className="text-[11px] font-medium uppercase tracking-wide text-(--text-tertiary)">Focus score</p>
                  <p className="mt-1.5 text-xl font-semibold tracking-tight">
                    {s.productivityScore != null ? `${s.productivityScore.toFixed(0)}/100` : "—"}
                  </p>
                  <p className="mt-1 text-[11px] text-(--text-tertiary)">{s.deepWorkSessions} deep work sessions</p>
                </CardContent>
              </Card>
            </div>
          </div>

          {/* Insights */}
          <div className="grid gap-5 sm:grid-cols-3">
            <Card>
              <CardContent className="px-5 py-4">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--text-tertiary)">Distraction index</p>
                <p className="mt-1.5 text-xl font-semibold tracking-tight">
                  {s.distractionIndex != null ? s.distractionIndex.toFixed(1) : "—"}
                </p>
                <p className="mt-1 text-[11px] text-(--text-tertiary)">app switches / active hour</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="px-5 py-4">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--text-tertiary)">Deep work</p>
                <p className="mt-1.5 text-xl font-semibold tracking-tight">{formatDuration(s.deepWorkTimeMs)}</p>
                <p className="mt-1 text-[11px] text-(--text-tertiary)">
                  {s.deepWorkSessions} session{s.deepWorkSessions === 1 ? "" : "s"} (&gt;15m in dev/productivity)
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="px-5 py-4">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--text-tertiary)">Goal streak</p>
                <p className="mt-1.5 text-xl font-semibold tracking-tight">
                  {s.goalStreakDays} day{s.goalStreakDays === 1 ? "" : "s"}
                </p>
                <p className="mt-1 text-[11px] text-(--text-tertiary)">
                  {s.goalMet ? "Today’s goal met 🎉" : "Today’s goal not yet met"}
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Hourly timeline */}
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Today</CardTitle>
                <CardDescription>Screen activity by hour</CardDescription>
              </div>
            </CardHeader>
            <CardContent>
              <HourlyTimeline buckets={buckets} />
            </CardContent>
          </Card>

          {/* Category doughnut */}
          {pieData.length > 0 && (
            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Time by category</CardTitle>
                  <CardDescription>Where your minutes went</CardDescription>
                </div>
              </CardHeader>
              <CardContent>
                <div className="flex justify-center pt-2">
                  <DonutChart data={slices} centerLabel={formatDuration(s.totalScreenTimeMs)} centerSub="screen time" />
                </div>
                <div className="mt-3 flex flex-wrap justify-center gap-x-4 gap-y-1.5 text-[11px] text-(--text-secondary)">
                  {slices.map((s, i) => (
                    <span key={i} className="inline-flex items-center gap-1.5">
                      <i className="size-2 rounded-full" style={{ background: s.color }} />
                      {s.label.replace("_", " ")} · {formatDuration(s.value)}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Top apps / sites — graceful empty states until Reporting is wired */}
          <div className="grid gap-5 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Top applications</CardTitle>
              </CardHeader>
              <CardContent>
                {apps.length === 0 ? (
                  <EmptyList hint="Apps you spend the most time in will appear here." />
                ) : (
                  <ul className="divide-y divide-(--border-subdued)">
                    {apps.slice(0, 6).map((a) => (
                      <li
                        key={a.process_name}
                        className="group flex items-center gap-2 py-2.5 transition-colors hover:bg-(--surface-2) hover:px-2"
                      >
                        <span className="size-2 shrink-0 rounded-full" style={{ background: categoryColor(a.category) }} />
                        <span className="truncate text-sm font-medium">{a.process_name}</span>
                        <span className="ml-auto shrink-0 text-xs text-(--text-tertiary) group-hover:text-(--text-secondary)">
                          {formatDuration(a.total_ms)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Top sites</CardTitle>
              </CardHeader>
              <CardContent>
                {sites.length === 0 ? (
                  <EmptyList hint="Websites you visit most will appear here." />
                ) : (
                  <ul className="divide-y divide-(--border-subdued)">
                    {sites.slice(0, 6).map((s) => (
                      <li
                        key={s.domain}
                        className="group flex items-center justify-between gap-2 py-2.5 transition-colors hover:bg-(--surface-2) hover:px-2"
                      >
                        <span className="truncate text-sm font-medium">{s.domain}</span>
                        <span className="shrink-0 text-xs text-(--text-tertiary) group-hover:text-(--text-secondary)">
                          {formatDuration(s.total_ms)} · {s.visits} visits
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </div>
        </>
      )}
    </div>
  );
}

function EmptyList({ hint }: { hint: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-(--border-subdued) py-10 text-center">
      <div className="flex size-10 items-center justify-center rounded-full bg-(--surface-2) text-(--text-tertiary)">
        <Icon name="dashboard-no-data" size={20} />
      </div>
      <p className="max-w-[220px] text-xs leading-relaxed text-(--text-tertiary)">{hint}</p>
    </div>
  );
}
