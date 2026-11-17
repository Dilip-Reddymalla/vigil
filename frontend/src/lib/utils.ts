import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Format a millisecond duration as "5h 24m" / "12m 30s". */
export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || ms < 0) return "—";
  const totalSeconds = Math.round(ms / 1000);
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

/** Format bytes into a human-readable unit. */
export function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

const CATEGORY_COLORS: Record<string, string> = {
  productivity: "var(--cat-productivity)",
  development: "var(--cat-development)",
  communication: "var(--cat-communication)",
  entertainment: "var(--cat-entertainment)",
  social_media: "var(--cat-social)",
  browsing: "var(--cat-browsing)",
  system: "var(--cat-system)",
};

export function categoryColor(category: string): string {
  return CATEGORY_COLORS[category] ?? "var(--cat-system)";
}

export function categoryLabel(category: string): string {
  return category.replace("_", " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
