import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { DashboardSummary, ServiceStatus } from "@/types";
import { today } from "./date";
import { log } from "./logger";

export { today };

function num(v: unknown): number {
  return typeof v === "number" ? v : 0;
}
function numOrNull(v: unknown): number | null {
  return typeof v === "number" ? v : null;
}

function mapSummary(raw: any): DashboardSummary {
  return {
    localDate: String(raw.local_date ?? ""),
    totalScreenTimeMs: num(raw.total_screen_time_ms),
    activeTimeMs: num(raw.active_time_ms),
    passiveTimeMs: num(raw.passive_time_ms),
    idleTimeMs: num(raw.idle_time_ms),
    lockedTimeMs: num(raw.locked_time_ms),
    sleepTimeMs: num(raw.sleep_time_ms),
    monitorOffTimeMs: num(raw.monitor_off_time_ms),
    sessionCount: num(raw.session_count),
    hourlyBreakdownMs: Array.isArray(raw.hourly_breakdown_ms)
      ? raw.hourly_breakdown_ms.map(num)
      : Array(24).fill(0),
    productivityScore: numOrNull(raw.productivity_score),
    deepWorkSessions: num(raw.deep_work_sessions),
    deepWorkTimeMs: num(raw.deep_work_time_ms),
    appSwitches: num(raw.app_switches),
    distractionIndex: numOrNull(raw.distraction_index),
    goalMet: Boolean(raw.goal_met),
    goalStreakDays: num(raw.goal_streak_days),
    goalMs: num(raw.goal_ms) || 8 * 3_600_000,
    networkSentBytes: num(raw.network_sent_bytes),
    networkRecvBytes: num(raw.network_recv_bytes),
  };
}

/** Internal invoke wrapper that logs every call + error. */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  log.info(`invoke ${cmd}`, args ?? {});
  try {
    const r = await invoke<T>(cmd, args);
    log.info(`invoke ${cmd} => ok`, r);
    return r;
  } catch (e) {
    log.error(`invoke ${cmd} => FAILED`, e);
    throw e;
  }
}

export async function fetchSummary(localDate: string = today()): Promise<DashboardSummary> {
  const raw = await call<any>("get_dashboard", { localDate });
  const mapped = mapSummary(raw);
  log.info("fetchSummary mapped", mapped);
  return mapped;
}

export interface TopApp {
  process_name: string;
  category: string;
  total_ms: number;
  sessions: number;
}
export async function fetchApps(localDate: string = today()): Promise<TopApp[]> {
  try {
    return await call<TopApp[]>("applications_top", { localDate });
  } catch {
    return [];
  }
}

export interface TopSite {
  domain: string;
  visits: number;
  total_ms: number;
}
export async function fetchSites(localDate: string = today()): Promise<TopSite[]> {
  try {
    return await call<TopSite[]>("websites_top", { localDate });
  } catch {
    return [];
  }
}

export interface HourlyNetwork {
  hour: number;
  sent: number;
  recv: number;
}
export async function fetchNetworkHourly(localDate: string = today()): Promise<HourlyNetwork[]> {
  try {
    return await call<HourlyNetwork[]>("network_hourly", { localDate });
  } catch {
    return [];
  }
}

export interface SessionRow {
  process_name: string;
  category: string;
  start_time: string;
  duration_ms: number;
}
export async function fetchTimeline(localDate: string = today()): Promise<SessionRow[]> {
  try {
    return await call<SessionRow[]>("timeline_sessions", { localDate });
  } catch {
    return [];
  }
}

function mapStatus(raw: any): ServiceStatus {
  return {
    version: String(raw.version ?? ""),
    uptimeSeconds: num(raw.uptime_seconds),
    systemState: raw.system_state ?? "unknown",
    activityState: raw.activity_state ?? "unknown",
    displayState: raw.display_state ?? "unknown",
    activeSession: raw.active_session ?? null,
    dbSizeBytes: num(raw.db_size_bytes),
    isPaused: Boolean(raw.is_paused),
  };
}

export async function getStatus(): Promise<ServiceStatus | null> {
  try {
    const raw = await call<any>("get_status");
    return raw ? mapStatus(raw) : null;
  } catch {
    return null;
  }
}

export async function setPaused(paused: boolean): Promise<boolean> {
  try {
    return await call<boolean>("set_paused", { paused });
  } catch {
    return false;
  }
}

export async function setSetting(key: string, value: string, valueType: string): Promise<boolean> {
  try {
    return await call<boolean>("set_setting", { key, value, valueType });
  } catch {
    return false;
  }
}

export async function getSetting(key: string): Promise<string | null> {
  try {
    return await call<string | null>("settings_get", { key });
  } catch {
    return null;
  }
}

export async function getSystemName(): Promise<string> {
  try {
    const v = await call<unknown>("system_name");
    return typeof v === "string" && v.length > 0 ? v : "This PC";
  } catch {
    return "This PC";
  }
}

export interface UpdateInfo {
  installed: string;
  latest: string;
  has_update: boolean;
  assets: string[];
}

export async function checkForUpdates(owner: string, repo: string): Promise<UpdateInfo | null> {
  try {
    return await call<UpdateInfo>("update_check", { owner, repo });
  } catch {
    return null;
  }
}

export async function applyUpdate(owner: string, repo: string): Promise<boolean> {
  try {
    return await call<boolean>("update_apply", { owner, repo });
  } catch {
    return false;
  }
}

export async function exitApp(): Promise<void> {
  try {
    await call("app_exit");
  } catch {
    /* ignore */
  }
}

export interface ComparisonMetrics {
  screen_ms: number;
  focus: number | null;
  deep_ms: number;
  deep_sessions: number;
  distraction: number | null;
  active_ratio: number | null;
  days: number;
}
export interface ComparisonDailyPoint {
  date: string;
  screen_ms: number;
  deep_ms: number;
  focus: number | null;
}
export type Verdict = "good" | "mixed" | "needs_attention";
export interface Comparison {
  period: string;
  verdict: Verdict;
  reasons: string[];
  current: ComparisonMetrics;
  previous: ComparisonMetrics;
  deltas: {
    screen_pct: number | null;
    focus_pct: number | null;
    deep_pct: number | null;
    deep_sessions_pct: number | null;
    distraction_pct: number | null;
    active_pct: number | null;
  };
  daily: { current: ComparisonDailyPoint[]; previous: ComparisonDailyPoint[] };
}

export async function fetchComparison(period: "week" | "month"): Promise<Comparison | null> {
  try {
    return await call<Comparison>("overview_compare", { period });
  } catch {
    return null;
  }
}

export async function addExclusion(processName: string): Promise<boolean> {
  try {
    return await call<boolean>("add_exclusion", { processName });
  } catch {
    return false;
  }
}

export async function listExclusions(): Promise<string[]> {
  try {
    return await call<string[]>("exclusions_list");
  } catch {
    return [];
  }
}

export async function backupDb(targetPath: string): Promise<boolean> {
  try {
    return await call<boolean>("backup_db", { targetPath });
  } catch {
    return false;
  }
}

export async function exportReport(
  anchor: string,
  targetPath: string,
  granularity: "daily" | "weekly" | "monthly" = "daily",
  format: "csv" | "pdf" = "csv",
): Promise<string> {
  try {
    return await call<string>("export_report", { anchor, targetPath, granularity, format });
  } catch (e) {
    log.error("export_report failed", e);
    return "";
  }
}

export async function setRetention(days: number): Promise<boolean> {
  try {
    return await call<boolean>("database_retention", { keepDays: days });
  } catch (e) {
    log.error("set_retention failed", e);
    return false;
  }
}

export async function getRetention(): Promise<number> {
  try {
    const v = await call<any>("database_retention_get");
    return Number(v?.retention_days ?? 90);
  } catch {
    return 90;
  }
}

/** Open the compiled folder in Explorer via the Tauri command. */
export async function openReportFolder(): Promise<boolean> {
  try {
    return await call<boolean>("open_report_folder");
  } catch (e) {
    log.error("open_report_folder failed", e);
    return false;
  }
}

/** Open a native folder picker; returns the chosen directory (and null if cancelled). */
export async function pickFolder(): Promise<string | null> {
  try {
    const dir = await open({
      directory: true,
      multiple: false,
      title: "Choose report export folder",
    });
    return typeof dir === "string" && dir.length > 0 ? dir : null;
  } catch (e) {
    log.error("pickFolder failed", e);
    return null;
  }
}
