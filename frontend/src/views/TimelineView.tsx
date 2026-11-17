import { useQuery } from "@tanstack/react-query";
import { fetchTimeline } from "@/lib/api";
import { useDataLogger } from "@/hooks/useDataLogger";
import { categoryColor, categoryLabel, formatDuration } from "@/lib/utils";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useSelectedDate } from "@/lib/date";

function fmtStart(iso: string) {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function fmtEnd(iso: string, ms: number) {
  const d = new Date(new Date(iso).getTime() + ms);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function TimelineView() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data: sessions = [], isLoading } = useQuery({
    queryKey: ["timeline", date, refreshEpoch],
    queryFn: () => fetchTimeline(date),
    refetchInterval: 5_000,
  });
  useDataLogger("TimelineView.sessions", sessions, { isLoading });

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Timeline</h1>
        <p className="text-sm text-(--text-secondary)">Today’s session sequence</p>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Session flow</CardTitle>
            <CardDescription>{sessions.length} sessions today</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="py-10 text-center text-sm text-(--text-secondary)">Loading…</div>
          ) : sessions.length === 0 ? (
            <div className="rounded-xl border border-dashed border-(--border-subdued) py-12 text-center">
              <p className="text-sm font-medium text-(--text-secondary)">Nothing recorded yet</p>
              <p className="mt-1 text-xs text-(--text-tertiary)">Your app sessions will appear here in order.</p>
            </div>
          ) : (
            <ol className="relative ml-3 space-y-4 border-l border-(--border-subdued) pl-5">
              {sessions.map((s, i) => (
                <li key={i} className="relative">
                  <span
                    className="absolute -left-[29px] top-1.5 size-3 rounded-full ring-4 ring-(--surface)"
                    style={{ background: categoryColor(s.category) }}
                  />
                  <div className="flex flex-wrap items-baseline justify-between gap-2">
                    <p className="text-sm font-medium">{s.process_name}</p>
                    <p className="text-xs text-(--text-tertiary)">
                      {fmtStart(s.start_time)} – {fmtEnd(s.start_time, s.duration_ms)} · {formatDuration(s.duration_ms)}
                    </p>
                  </div>
                  <p className="text-[11px] text-(--text-tertiary)">{categoryLabel(s.category)}</p>
                </li>
              ))}
            </ol>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
