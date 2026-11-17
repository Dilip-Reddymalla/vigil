import { useQuery } from "@tanstack/react-query";
import { fetchSummary, getStatus } from "@/lib/api";
import { useSelectedDate } from "@/lib/date";

/** Live connectivity + data banner shown above the routed content. */
export function ConnectionBanner() {
  const { date, refreshEpoch } = useSelectedDate();
  const { data: status } = useQuery({
    queryKey: ["status", refreshEpoch],
    queryFn: () => getStatus(),
    refetchInterval: 10_000,
  });
  const { data: summary, error: summaryError } = useQuery({
    queryKey: ["summary", date, refreshEpoch],
    queryFn: () => fetchSummary(date),
    refetchInterval: 15_000,
  });

  const online = !!status;
  const dataError = summaryError instanceof Error ? summaryError.message : summaryError ? String(summaryError) : null;
  const bad = !online || !!dataError;

  return (
    <div
      className={
        "mx-auto mb-4 flex w-full max-w-6xl items-center gap-2 rounded-xl px-4 py-2 text-xs font-medium ring-1 " +
        (bad
          ? "bg-(--cat-social)/10 text-(--cat-social) ring-(--cat-social)/20"
          : "bg-(--cat-productivity)/10 text-(--cat-productivity) ring-(--cat-productivity)/20")
      }
    >
      <span className={"size-2 shrink-0 rounded-full " + (bad ? "bg-(--cat-social)" : "bg-(--cat-productivity)")} />
      {dataError ? (
        <span>Data error: {dataError}</span>
      ) : online ? (
        <span>
          Service connected · uptime {status?.uptimeSeconds ?? 0}s · {summary?.sessionCount ?? 0} sessions today
        </span>
      ) : (
        <span>Service offline — start vigil-service.exe and relaunch the dashboard</span>
      )}
    </div>
  );
}
