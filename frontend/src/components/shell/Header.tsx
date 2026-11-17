import { useRef } from "react";
import { RefreshCw } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { useTheme } from "@/lib/theme";
import { useSelectedDate, shiftDate, formatFriendly } from "@/lib/date";
import { log } from "@/lib/logger";

function windowApi() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export function Header() {
  const { theme, setTheme } = useTheme();
  const { date, setDate, bumpRefresh } = useSelectedDate();
  const dateInputRef = useRef<HTMLInputElement>(null);
  const isDark = theme === "dark";
  const toggle = () => setTheme(isDark ? "light" : "dark");
  const refresh = () => {
    log.info("manual refresh requested");
    bumpRefresh(); // increments refreshEpoch → all queries re-key → guaranteed refetch
  };

  const onMinimize = () => windowApi()?.minimize();
  const onMaximize = () => windowApi()?.toggleMaximize();
  const onClose = () => windowApi()?.close();

  return (
    <header
      data-tauri-drag-region
      className="flex h-14 items-center justify-between border-b border-(--border-subdued) bg-(--background)/80 pl-5 backdrop-blur"
    >
      {/* Date navigator */}
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="icon" aria-label="Previous day" onClick={() => setDate(shiftDate(date, -1))}>
          <Icon name="date-previous" size={18} />
        </Button>
        <button
          type="button"
          onClick={() => dateInputRef.current?.showPicker()}
          className="min-w-[110px] cursor-pointer rounded-md px-2 py-1 text-center text-sm font-medium transition-colors hover:bg-(--surface-2)"
          title="Pick a date"
        >
          {formatFriendly(date)}
        </button>
        <input
          ref={dateInputRef}
          type="date"
          value={date}
          onChange={(e) => e.target.value && setDate(e.target.value)}
          className="sr-only h-0 w-0"
          tabIndex={-1}
          aria-hidden="true"
        />
        <Button variant="ghost" size="icon" aria-label="Next day" onClick={() => setDate(shiftDate(date, 1))}>
          <Icon name="date-next" size={18} />
        </Button>
      </div>

      <div className="flex items-center gap-1">
        <span className="mr-2 hidden items-center gap-1.5 rounded-full bg-(--surface) px-3 py-1 text-xs text-(--text-secondary) ring-1 ring-(--border-subdued) sm:flex">
          <Icon name="service-running" size={14} className="text-(--cat-productivity)" />
          Service running
        </span>
        <Button variant="ghost" size="icon" aria-label="Refresh data" onClick={refresh}>
          <RefreshCw className="size-[18px]" />
        </Button>
        <Button variant="ghost" size="icon" aria-label="Toggle theme" onClick={toggle}>
          <Icon name={isDark ? "theme-light" : "theme-dark"} size={18} />
        </Button>

        {/* Window controls (frameless title bar) */}
        <div className="ml-1 flex items-center border-l border-(--border-subdued) pl-1">
          <button
            type="button"
            onClick={onMinimize}
            aria-label="Minimize"
            className="grid h-9 w-11 place-items-center rounded-lg text-(--text-secondary) transition-colors hover:bg-black/5 dark:hover:bg-white/10"
          >
            <Icon name="window-minimize" size={16} />
          </button>
          <button
            type="button"
            onClick={onMaximize}
            aria-label="Maximize"
            className="grid h-9 w-11 place-items-center rounded-lg text-(--text-secondary) transition-colors hover:bg-black/5 dark:hover:bg-white/10"
          >
            <Icon name="window-maximize" size={16} />
          </button>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="grid h-9 w-11 place-items-center rounded-lg text-(--text-secondary) transition-colors hover:bg-red-500 hover:text-white"
          >
            <Icon name="window-close" size={16} />
          </button>
        </div>
      </div>
    </header>
  );
}
