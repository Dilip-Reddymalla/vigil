import { useQuery } from "@tanstack/react-query";
import { fetchSummary } from "@/lib/api";
import { EMPTY_SUMMARY } from "@/types";

export function useDashboard(date: string, refreshEpoch: number) {
  return useQuery({
    queryKey: ["dashboard", date, refreshEpoch],
    queryFn: () => fetchSummary(date),
    staleTime: 5_000,
    refetchInterval: 5_000,
    placeholderData: EMPTY_SUMMARY,
  });
}
