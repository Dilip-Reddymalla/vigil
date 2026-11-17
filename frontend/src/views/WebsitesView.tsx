import { useQuery } from "@tanstack/react-query";
import { fetchSites } from "@/lib/api";
import { useDataLogger } from "@/hooks/useDataLogger";
import { formatDuration } from "@/lib/utils";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useSelectedDate } from "@/lib/date";

export function WebsitesView() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data: sites = [], isLoading } = useQuery({
    queryKey: ["sites", date, refreshEpoch],
    queryFn: () => fetchSites(date),
    refetchInterval: 5_000,
  });
  useDataLogger("WebsitesView.sites", sites, { isLoading });
  const max = Math.max(1, ...sites.map((s) => s.total_ms));

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Websites</h1>
        <p className="text-sm text-(--text-secondary)">Domains you visited today</p>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Top sites</CardTitle>
            <CardDescription>Domain-level activity (privacy-preserving)</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="py-10 text-center text-sm text-(--text-secondary)">Loading…</div>
          ) : sites.length === 0 ? (
            <EmptyState />
          ) : (
            <ul className="divide-y divide-(--border-subdued)">
              {sites.map((s) => (
                <li
                  key={s.domain}
                  className="group flex items-center gap-4 rounded-lg py-3 transition-colors hover:bg-(--surface-2) hover:px-2"
                >
                  <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-(--surface-2) text-xs font-semibold text-(--text-secondary)">
                    {s.domain.slice(0, 1).toUpperCase()}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <p className="truncate text-sm font-medium">{s.domain}</p>
                      <p className="text-sm font-semibold">{formatDuration(s.total_ms)}</p>
                    </div>
                    <div className="mt-1 flex items-center gap-2">
                      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-(--surface-2)">
                        <div
                          className="h-full rounded-full bg-(--cat-browsing)"
                          style={{ width: `${(s.total_ms / max) * 100}%` }}
                        />
                      </div>
                      <span className="shrink-0 text-[11px] text-(--text-tertiary)">
                        {s.visits} visit{s.visits === 1 ? "" : "s"}
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
      <p className="text-sm font-medium text-(--text-secondary)">No site visits recorded yet</p>
      <p className="max-w-[240px] text-xs text-(--text-tertiary)">
        Domains you browse will appear here (only domain names — never full URLs by default).
      </p>
    </div>
  );
}
