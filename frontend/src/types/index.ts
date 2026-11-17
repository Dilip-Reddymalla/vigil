/** Shared API types mirroring the IPC protocol (`dashboard.get_summary`). */
export interface DashboardSummary {
  localDate: string;
  totalScreenTimeMs: number;
  activeTimeMs: number;
  passiveTimeMs: number;
  idleTimeMs: number;
  lockedTimeMs: number;
  sleepTimeMs: number;
  monitorOffTimeMs: number;
  sessionCount: number;
  hourlyBreakdownMs: number[];
  productivityScore: number | null;
  deepWorkSessions: number;
  deepWorkTimeMs: number;
  appSwitches: number;
  distractionIndex: number | null;
  goalMet: boolean;
  goalStreakDays: number;
  goalMs: number;
  networkSentBytes: number;
  networkRecvBytes: number;
}

export const EMPTY_SUMMARY: DashboardSummary = {
  localDate: "",
  totalScreenTimeMs: 0,
  activeTimeMs: 0,
  passiveTimeMs: 0,
  idleTimeMs: 0,
  lockedTimeMs: 0,
  sleepTimeMs: 0,
  monitorOffTimeMs: 0,
  sessionCount: 0,
  hourlyBreakdownMs: Array(24).fill(0),
  productivityScore: null,
  deepWorkSessions: 0,
  deepWorkTimeMs: 0,
  appSwitches: 0,
  distractionIndex: null,
  goalMet: false,
  goalStreakDays: 0,
  goalMs: 8 * 3_600_000,
  networkSentBytes: 0,
  networkRecvBytes: 0,
};

export interface ServiceStatus {
  version: string;
  uptimeSeconds: number;
  systemState: "unlocked" | "locked" | "sleeping";
  activityState: "active" | "passive" | "idle";
  displayState: "on" | "off";
  activeSession: {
    sessionUuid: string;
    processName: string;
    windowTitle: string;
    category: string;
    startTime: string;
  } | null;
  dbSizeBytes: number;
  isPaused: boolean;
}
