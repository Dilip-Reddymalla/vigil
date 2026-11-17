import { useQuery } from "@tanstack/react-query";
import { fetchApps } from "@/lib/api";
import { useDataLogger } from "@/hooks/useDataLogger";
import { categoryColor, categoryLabel, formatDuration } from "@/lib/utils";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useSelectedDate } from "@/lib/date";

export function ApplicationsView() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data: apps = [], isLoading } = useQuery({
    queryKey: ["apps", date, refreshEpoch],
    queryFn: () => fetchApps(date),
    refetchInterval: 5_000,
  });
  useDataLogger("ApplicationsView.apps", apps, { isLoading });
  const max = Math.max(1, ...apps.map((a) => a.total_ms));

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Applications</h1>
        <p className="text-sm text-(--text-secondary)">Where your time goes today</p>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Top applications</CardTitle>
            <CardDescription>{apps.length} tracked today</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="py-10 text-center text-sm text-(--text-secondary)">Loading…</div>
          ) : apps.length === 0 ? (
            <EmptyState />
          ) : (
            <ul className="divide-y divide-(--border-subdued)">
              {apps.map((a) => (
                <li
                  key={a.process_name}
                  className="group flex items-center gap-4 rounded-lg py-3 transition-colors hover:bg-(--surface-2) hover:px-2"
                >
                  <span className="size-2.5 shrink-0 rounded-full" style={{ background: categoryColor(a.category) }} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <p className="truncate text-sm font-medium">{a.process_name}</p>
                      <p className="text-sm font-semibold">{formatDuration(a.total_ms)}</p>
                    </div>
                    <div className="mt-1 flex items-center gap-2">
                      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-(--surface-2)">
                        <div
                          className="h-full rounded-full"
                          style={{ width: `${(a.total_ms / max) * 100}%`, background: categoryColor(a.category) }}
                        />
                      </div>
                      <span className="shrink-0 text-[11px] text-(--text-tertiary)">
                        {categoryLabel(a.category)} · {a.sessions} session{a.sessions === 1 ? "" : "s"}
                      </span>
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-(--border-subdued) py-12 text-center">
      <p className="text-sm font-medium text-(--text-secondary)">No app activity recorded yet</p>
      <p className="max-w-[240px] text-xs text-(--text-tertiary)">
        Apps you use will show up here as you work.
      </p>
    </div>
  );
}
