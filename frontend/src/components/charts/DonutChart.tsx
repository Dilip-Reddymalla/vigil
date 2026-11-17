import { useState } from "react";
import { cn } from "@/lib/utils";
import { formatDuration } from "@/lib/utils";

export interface DonutSlice {
  label: string;
  value: number;
  color: string;
}

/**
 * SVG doughnut with per-slice hover: the hovered slice brightens, expands, and a
 * tooltip shows the label + duration.
 */
export function DonutChart({
  data,
  size = 200,
  thickness = 24,
  centerLabel,
  centerSub,
  className,
}: {
  data: DonutSlice[];
  size?: number;
  thickness?: number;
  centerLabel?: string;
  centerSub?: string;
  className?: string;
}) {
  const [hover, setHover] = useState<number | null>(null);
  const total = data.reduce((a, b) => a + b.value, 0);
  if (total <= 0) return null;

  const cx = size / 2;
  const r = (size - thickness) / 2;
  const circ = 2 * Math.PI * r;
  let acc = 0;

  return (
    <div className={cn("relative inline-grid place-items-center", className)} style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {data.map((d, i) => {
          const frac = d.value / total;
          const len = frac * circ;
          const start = acc;
          acc += len;
          const isHover = hover === i;
          return (
            <circle
              key={i}
              cx={cx}
              cy={cx}
              r={r}
              fill="none"
              stroke={d.color}
              strokeWidth={isHover ? thickness + 5 : thickness}
              strokeLinecap="butt"
              strokeDasharray={`${Math.max(0, len - 2)} ${circ - len + 2}`}
              strokeDashoffset={-start}
              transform={`rotate(-90 ${cx} ${cx})`}
              className="cursor-pointer transition-all duration-200 ease-out"
              opacity={hover === null ? 1 : isHover ? 1 : 0.35}
              style={{ filter: isHover ? "brightness(1.15)" : undefined }}
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover(null)}
            />
          );
        })}
      </svg>

      {/* Center label */}
      <div className="pointer-events-none absolute inset-0 grid place-items-center text-center">
        <div>
          <p className="text-xl font-semibold tracking-tight">{centerLabel ?? formatDuration(total)}</p>
          {centerSub && <p className="text-[10px] text-(--text-tertiary)">{centerSub}</p>}
        </div>
      </div>

      {/* Hover tooltip */}
      {hover !== null && (
        <div className="pointer-events-none absolute -top-9 left-1/2 z-20 -translate-x-1/2 rounded-md bg-(--text-primary) px-2.5 py-1 text-[11px] font-medium text-(--background) shadow">
          {data[hover].label.replace("_", " ")} · {formatDuration(data[hover].value)}
        </div>
      )}
    </div>
  );
}
