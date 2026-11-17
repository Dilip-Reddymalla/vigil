import { useQuery } from "@tanstack/react-query";
import { fetchNetworkHourly, fetchSummary } from "@/lib/api";
import { useDataLogger } from "@/hooks/useDataLogger";
import { formatBytes } from "@/lib/utils";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useSelectedDate } from "@/lib/date";

export function NetworkView() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data: summary } = useQuery({
    queryKey: ["summary", date, refreshEpoch],
    queryFn: () => fetchSummary(date),
    refetchInterval: 5_000,
  });
  useDataLogger("NetworkView.summary", summary);
  const { data: hourlyRaw = [] } = useQuery({
    queryKey: ["network", date, refreshEpoch],
    queryFn: () => fetchNetworkHourly(date),
    refetchInterval: 5_000,
  });
  // Guard against malformed/empty payloads so the view never throws.
  const hourly = Array.isArray(hourlyRaw) ? hourlyRaw : [];
  useDataLogger("NetworkView.hourly", hourly);

  // sqrt scale so small bars stay visible even when one hour dominates
  const max = Math.max(1, ...hourly.map((h) => Math.max(Number(h.sent) || 0, Number(h.recv) || 0)));
  const heightPct = (v: number) => Math.max(8, Math.round(Math.sqrt(v / max) * 100));
  const sent = summary?.networkSentBytes ?? 0;
  const recv = summary?.networkRecvBytes ?? 0;

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Network</h1>
        <p className="text-sm text-(--text-secondary)">Interface-level traffic today</p>
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Uploaded</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-semibold text-(--accent)">{formatBytes(sent)}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Downloaded</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-semibold text-(--accent-2)">{formatBytes(recv)}</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Hourly traffic</CardTitle>
            <CardDescription>↑ sent / ↓ received per hour</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex h-40 items-end gap-[3px]">
            {hourly.map((h) => (
              <div key={h.hour} className="group relative flex h-full flex-1 flex-col justify-end gap-[2px]">
                <div className="pointer-events-none absolute inset-x-0 -top-9 z-20 mx-auto w-max rounded-md bg-(--text-primary) px-2 py-1 text-[10px] font-medium text-(--background) opacity-0 shadow transition-opacity group-hover:opacity-100">
                  {h.hour}:00 · ↑ {formatBytes(h.sent)} · ↓ {formatBytes(h.recv)}
                </div>
                <div
                  className="rounded-t-[3px] bg-(--accent)/70 transition-colors group-hover:bg-(--accent)"
                  style={{ height: `${heightPct(h.sent)}%` }}
                  title={`${h.hour}:00 sent ${formatBytes(h.sent)}`}
                />
                <div
                  className="rounded-b-[3px] bg-(--accent-2)/70 transition-colors group-hover:bg-(--accent-2)"
                  style={{ height: `${heightPct(h.recv)}%` }}
                  title={`${h.hour}:00 recv ${formatBytes(h.recv)}`}
                />
              </div>
            ))}
          </div>
          <div className="mt-1.5 flex gap-[3px]">
            {hourly.map((h) => (
              <span key={h.hour} className="flex-1 text-center text-[8px] font-medium leading-none text-(--text-tertiary)">
                {String(h.hour).padStart(2, "0")}
              </span>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
