import { invoke } from "@tauri-apps/api/core";

/**
 * Structured logger for the dashboard UI.
 * - Console (always)
 * - On-screen panel when opened with `?debug`
 * - Persistent file via the `dashboard_log` Tauri command (when running in Tauri)
 */

const TAG = "[Vigil]";
const MAX_PANEL_LINES = 200;
const STORAGE_KEY = "vigil.dashboardLogging";

let logEnabled = true;
try {
  const v = localStorage.getItem(STORAGE_KEY);
  if (v === "false") logEnabled = false;
} catch {
  /* ignore */
}

/** Enable/disable all dashboard logging (console, on-screen panel, file). */
export function setLogEnabled(enabled: boolean) {
  logEnabled = enabled;
  try {
    localStorage.setItem(STORAGE_KEY, String(enabled));
  } catch {
    /* ignore */
  }
}

export function isLogEnabled() {
  return logEnabled;
}

function mirror(line: string) {
  try {
    if (new URLSearchParams(location.search).has("debug")) {
      let el = document.getElementById("vigil-debug-log");
      if (!el) {
        el = document.createElement("pre");
        el.id = "vigil-debug-log";
        el.style.cssText =
          "position:fixed;left:8px;bottom:8px;max-height:200px;width:64%;overflow:auto;" +
          "background:rgba(0,0,0,.86);color:#7ef0a0;font:11px/1.35 monospace;z-index:99999;" +
          "padding:8px 10px;border-radius:8px;pointer-events:auto;";
        document.body.appendChild(el);
      }
      const text = el.textContent + "\n" + line;
      el.textContent = text.split("\n").slice(-MAX_PANEL_LINES).join("\n");
    }
  } catch {
    /* mirror is best-effort */
  }
}

function toFile(level: string, context: string, data: unknown) {
  try {
    void invoke("dashboard_log", {
      level,
      message: `${context} ${data === undefined ? "" : safeStringify(data)}`,
    });
  } catch {
    /* not in Tauri / no IPC — fine */
  }
}

function safeStringify(v: unknown): string {
  try {
    if (typeof v === "string") return v;
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

function emit(level: "info" | "warn" | "error", context: string, data: unknown) {
  if (!logEnabled) return;
  const line = `${TAG} [${level}] ${context} ${data === undefined ? "" : safeStringify(data)}`;
  if (level === "error") {
    // eslint-disable-next-line no-console
    console.error(line);
  } else if (level === "warn") {
    // eslint-disable-next-line no-console
    console.warn(line);
  } else {
    // eslint-disable-next-line no-console
    console.log(line);
  }
  mirror(line);
  toFile(level, context, data);
}

export const log = {
  info(context: string, data?: unknown) {
    emit("info", context, data);
  },
  warn(context: string, data?: unknown) {
    emit("warn", context, data);
  },
  error(context: string, data?: unknown) {
    emit("error", context, data);
  },
};
