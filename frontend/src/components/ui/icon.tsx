import { useMemo } from "react";
import { cn } from "@/lib/utils";

// Explicit raw imports (Vite `?raw`) — guaranteed to inline the SVG strings.
import activeScreen from "../../assets/icons/active-screen.svg?raw";
import applications from "../../assets/icons/applications.svg?raw";
import backup from "../../assets/icons/backup.svg?raw";
import dashboardNoData from "../../assets/icons/dashboard-no-data.svg?raw";
import dashboard from "../../assets/icons/dashboard.svg?raw";
import dateNext from "../../assets/icons/date-next.svg?raw";
import datePrevious from "../../assets/icons/date-previous.svg?raw";
import emptyState from "../../assets/icons/empty-state.svg?raw";
import exportCsv from "../../assets/icons/export-csv.svg?raw";
import focusScore from "../../assets/icons/focus-score.svg?raw";
import idleTime from "../../assets/icons/idle-time.svg?raw";
import improvements from "../../assets/icons/improvements.svg?raw";
import localDatabase from "../../assets/icons/local-database.svg?raw";
import lockedSession from "../../assets/icons/locked-session.svg?raw";
import network from "../../assets/icons/network.svg?raw";
import pauseMonitoring from "../../assets/icons/pause-monitoring.svg?raw";
import privacyShield from "../../assets/icons/privacy-shield.svg?raw";
import refresh from "../../assets/icons/refresh.svg?raw";
import reports from "../../assets/icons/reports.svg?raw";
import serviceRunning from "../../assets/icons/service-running.svg?raw";
import settings from "../../assets/icons/settings.svg?raw";
import sidebarCollapse from "../../assets/icons/sidebar-collapse.svg?raw";
import sidebarExpand from "../../assets/icons/sidebar-expand.svg?raw";
import sleepState from "../../assets/icons/sleep-state.svg?raw";
import themeDark from "../../assets/icons/theme-dark.svg?raw";
import themeLight from "../../assets/icons/theme-light.svg?raw";
import timeline from "../../assets/icons/timeline.svg?raw";
import vigilBrand from "../../assets/icons/vigil-brand.svg?raw";
import websites from "../../assets/icons/websites.svg?raw";
import windowClose from "../../assets/icons/window-close.svg?raw";
import windowMaximize from "../../assets/icons/window-maximize.svg?raw";
import windowMinimize from "../../assets/icons/window-minimize.svg?raw";

const ICONS: Record<string, string> = {
  "active-screen": activeScreen,
  applications,
  backup,
  "dashboard-no-data": dashboardNoData,
  dashboard,
  "date-next": dateNext,
  "date-previous": datePrevious,
  "empty-state": emptyState,
  "export-csv": exportCsv,
  "focus-score": focusScore,
  "idle-time": idleTime,
  improvements,
  "local-database": localDatabase,
  "locked-session": lockedSession,
  network,
  "pause-monitoring": pauseMonitoring,
  "privacy-shield": privacyShield,
  refresh,
  reports,
  "service-running": serviceRunning,
  settings,
  "sidebar-collapse": sidebarCollapse,
  "sidebar-expand": sidebarExpand,
  "sleep-state": sleepState,
  "theme-dark": themeDark,
  "theme-light": themeLight,
  timeline,
  "vigil-brand": vigilBrand,
  websites,
  "window-close": windowClose,
  "window-maximize": windowMaximize,
  "window-minimize": windowMinimize,
};

export function Icon({
  name,
  size = 20,
  className,
}: {
  name: string;
  size?: number;
  className?: string;
}) {
  const content = useMemo(() => ICONS[name], [name]);

  return (
    <span
      className={cn("inline-flex shrink-0 items-center justify-center", className)}
      style={{ width: size, height: size, color: "inherit" }}
    >
      {content ? (
        <span
          className="vigil-icon"
          style={{ width: size, height: size, display: "inline-flex" }}
          dangerouslySetInnerHTML={{ __html: content }}
        />
      ) : (
        <span
          className="rounded-full bg-current opacity-40"
          style={{ width: size * 0.35, height: size * 0.35 }}
        />
      )}
    </span>
  );
}
