import { createContext, useContext, useState, type ReactNode } from "react";

/** Local calendar date `YYYY-MM-DD` (matches how the service stores `local_date`). */
export const today = () => {
  const d = new Date();
  const yy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yy}-${mm}-${dd}`;
};

function toLocalDate(iso: string) {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** Return `iso` shifted by `delta` days, as `YYYY-MM-DD`. */
export function shiftDate(iso: string, delta: number): string {
  const d = toLocalDate(iso);
  d.setDate(d.getDate() + delta);
  const yy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yy}-${mm}-${dd}`;
}

export function formatFriendly(iso: string): string {
  const d = toLocalDate(iso);
  const today0 = toLocalDate(today());
  const diff = Math.round((d.getTime() - today0.getTime()) / 86_400_000);
  if (diff === 0) return "Today";
  if (diff === -1) return "Yesterday";
  if (diff === 1) return "Tomorrow";
  return d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
}

interface DateContextValue {
  date: string;
  setDate: (d: string) => void;
  /** Increments to force a guaranteed refetch across all views. */
  refreshEpoch: number;
  bumpRefresh: () => void;
}

const DateContext = createContext<DateContextValue>({
  date: today(),
  setDate: () => {},
  refreshEpoch: 0,
  bumpRefresh: () => {},
});

export function DateProvider({ children }: { children: ReactNode }) {
  const [date, setDate] = useState<string>(today());
  const [refreshEpoch, setRefreshEpoch] = useState(0);
  return (
    <DateContext.Provider
      value={{ date, setDate, refreshEpoch, bumpRefresh: () => setRefreshEpoch((e) => e + 1) }}
    >
      {children}
    </DateContext.Provider>
  );
}

export function useSelectedDate() {
  return useContext(DateContext);
}
