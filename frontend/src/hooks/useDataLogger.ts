import { useEffect, useRef } from "react";
import { log } from "@/lib/logger";

/**
 * Logs every data update and any error for a given query/view. Call from each
 * view with the query's data/error/isLoading so the dashboard log captures the
 * exact content being shown.
 */
export function useDataLogger(
  label: string,
  data: unknown,
  opts?: { isLoading?: boolean; error?: unknown },
) {
  const last = useRef<string>("__none__");

  useEffect(() => {
    if (opts?.isLoading) return;
    if (opts?.error !== undefined && opts?.error !== null) {
      log.error(`${label} error`, opts.error);
      return;
    }
    let summary = "";
    try {
      summary = JSON.stringify(data);
    } catch {
      summary = String(data);
    }
    if (summary !== last.current) {
      last.current = summary;
      log.info(`${label} update`, summary.length > 2000 ? `${summary.slice(0, 2000)}…` : summary);
    }
  }, [data, opts?.isLoading, opts?.error, label]);
}
