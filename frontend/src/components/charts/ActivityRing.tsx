import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface RingSegment {
  value: number; // 0..1
  color: string;
  label?: string;
}

const clamp01 = (n: number) => Math.max(0, Math.min(1, n));

/**
 * Apple-Watch-style concentric SVG rings. `size` is the outer px; each ring is
 * inset by `gap` from the previous. Hovering a ring highlights it.
 */
export function ActivityRing({
  size = 200,
  stroke = 11,
  gap = 6,
  segments,
  children,
  className,
}: {
  size?: number;
  stroke?: number;
  gap?: number;
  segments: RingSegment[];
  children?: ReactNode;
  className?: string;
}) {
  const [hover, setHover] = useState<number | null>(null);
  const center = size / 2;
  const maxR = (size - stroke) / 2;

  return (
    <div className={cn("relative inline-grid place-items-center", className)} style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="absolute inset-0">
        {segments.map((seg, i) => {
          const r = maxR - i * (stroke + gap);
          const c = 2 * Math.PI * r;
          const filled = clamp01(seg.value) * c;
          const isHover = hover === i;
          return (
            <g
              key={i}
              className="cursor-pointer"
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover(null)}
            >
              <circle
                cx={center}
                cy={center}
                r={r}
                fill="none"
                stroke="var(--border-subdued)"
                strokeWidth={stroke}
              />
              <circle
                cx={center}
                cy={center}
                r={r}
                fill="none"
                stroke={seg.color}
                strokeWidth={isHover ? stroke + 4 : stroke}
                strokeLinecap="round"
                strokeDasharray={`${filled} ${c}`}
                transform={`rotate(-90 ${center} ${center})`}
                opacity={hover === null ? 1 : isHover ? 1 : 0.4}
                style={{
                  transition: "stroke-dasharray 800ms cubic-bezier(.22,1,.36,1), opacity 200ms, stroke-width 200ms",
                  filter: isHover ? "brightness(1.12)" : undefined,
                }}
              />
            </g>
          );
        })}
      </svg>
      <div className="relative z-10 flex flex-col items-center text-center">{children}</div>
      {hover !== null && segments[hover]?.label && (
        <div className="pointer-events-none absolute -top-2 right-2 rounded-md bg-(--text-primary) px-2 py-0.5 text-[10px] font-medium text-(--background) shadow">
          {segments[hover].label}
        </div>
      )}
    </div>
  );
}
