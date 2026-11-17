import { cn } from "@/lib/utils";
import { formatDuration } from "@/lib/utils";

export interface HourBucket {
  label: string;
  active?: number; // 0..1 intensity
  category?: string;
  ms: number;
}

/**
 * Stacked 24-hour bar chart. Bars scale by sqrt(share of max) so small values stay
 * visible while the busiest hour still fills the container. Hover shows a tooltip.
 */
export function HourlyTimeline({ buckets, className }: { buckets: HourBucket[]; className?: string }) {
  const max = Math.max(1, ...buckets.map((b) => b.ms));
  return (
    <div className={cn("select-none", className)}>
      <div className="flex h-20 items-end gap-[3px]">
        {buckets.map((b, i) => {
          // sqrt scale: min ~10% so tiny values are still visible, max 100%
          const frac = Math.sqrt(b.ms / max);
          const pct = Math.max(10, Math.round(frac * 100));
          const isNow = i === new Date().getHours() && b.ms > 0;
          return (
            <div key={i} className="group relative flex h-full flex-1 flex-col justify-end">
              <div className="pointer-events-none absolute inset-x-0 -top-9 z-20 mx-auto w-max rounded-md bg-(--text-primary) px-2 py-1 text-[10px] font-medium text-(--background) opacity-0 shadow transition-opacity group-hover:opacity-100">
                {b.label}:00 · {formatDuration(b.ms)}
              </div>
              <div
                className={cn(
                  "relative w-full rounded-t-[5px]",
                  isNow && "ring-2 ring-(--accent)/50",
                )}
                style={{ height: `${pct}%` }}
              >
                <div
                  className="absolute inset-0 rounded-t-[5px] transition-colors"
                  style={{
                    background: b.category
                      ? `color-mix(in srgb, ${b.category} 85%, var(--surface))`
                      : "var(--accent)",
                    opacity: 0.9,
                  }}
                />
                <div
                  className="absolute inset-0 rounded-t-[5px] bg-black/0 transition-colors group-hover:bg-black/10 dark:group-hover:bg-white/10"
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* Per-hour labels */}
      <div className="mt-1.5 flex gap-[3px]">
        {buckets.map((b, i) => (
          <span key={i} className="flex-1 text-center text-[8px] font-medium leading-none text-(--text-tertiary)">
            {b.label}
          </span>
        ))}
      </div>
    </div>
  );
}
