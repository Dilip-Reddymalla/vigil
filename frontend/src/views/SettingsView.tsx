import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  addExclusion,
  applyUpdate,
  backupDb,
  checkForUpdates,
  exitApp,
  getRetention,
  getSetting,
  getStatus,
  getSystemName,
  listExclusions,
  setPaused,
  setRetention,
  setSetting,
  type UpdateInfo,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { isLogEnabled, setLogEnabled } from "@/lib/logger";

export function SettingsView() {
  const { data: status } = useQuery({ queryKey: ["status"], queryFn: () => getStatus() });
  const [paused, setPausedState] = useState(false);
  const [idle, setIdle] = useState("1200");
  const [goalHours, setGoalHours] = useState("8");
  const [retention, setRetentionState] = useState("90");
  const [exclusion, setExclusion] = useState("");
  const [exclusions, setExclusions] = useState<string[]>([]);
  const [backupPath, setBackupPath] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [loggingOn, setLoggingOn] = useState(isLogEnabled());
  const [exportDir, setExportDir] = useState("");
  const [displayName, setDisplayName] = useState("");
  const DEFAULT_UPDATE_OWNER = "Dilip-Reddymalla";
  const DEFAULT_UPDATE_REPO = "vigil";
  const [updateOwner, setUpdateOwner] = useState(DEFAULT_UPDATE_OWNER);
  const [updateRepo, setUpdateRepo] = useState(DEFAULT_UPDATE_REPO);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updateMsg, setUpdateMsg] = useState<string | null>(null);

  useEffect(() => {
    if (status) setPausedState(status.isPaused);
    getSetting("idle_threshold_seconds").then((v) => v && setIdle(v));
    getSetting("goal_hours").then((v) => v && setGoalHours(v));
    getSetting("export_dir").then((v) => v && setExportDir(v));
    getSystemName().then((n) => setDisplayName(n));
    getSetting("update_owner").then((v) => v && setUpdateOwner(v));
    getSetting("update_repo").then((v) => v && setUpdateRepo(v));
    setUpdateOwner((prev) => prev || DEFAULT_UPDATE_OWNER);
    setUpdateRepo((prev) => prev || DEFAULT_UPDATE_REPO);
    getRetention().then((d) => setRetentionState(String(d)));
    listExclusions().then(setExclusions);
  }, [status]);

  const togglePaused = async () => {
    const next = !paused;
    setPausedState(next);
    await setPaused(next);
    await setSetting("is_paused", String(next), "boolean");
  };

  const saveIdle = async () => {
    await setSetting("idle_threshold_seconds", idle, "integer");
    setMsg("Idle threshold saved.");
  };

  const saveGoal = async () => {
    const h = Number(goalHours);
    if (!Number.isFinite(h) || h <= 0) {
      setMsg("Enter a valid number of hours.");
      return;
    }
    await setSetting("goal_hours", String(Math.round(h)), "integer");
    setMsg(`Daily screen-time goal set to ${Math.round(h)}h.`);
  };

  const saveRetention = async () => {
    const d = Number(retention);
    if (!Number.isFinite(d) || d <= 0) {
      setMsg("Enter a valid number of days.");
      return;
    }
    const ok = await setRetention(Math.round(d));
    setMsg(ok ? `Retention set to ${Math.round(d)} days (old data purged).` : "Failed to set retention.");
  };

  const addExclusionFn = async () => {
    const name = exclusion.trim();
    if (!name) return;
    await addExclusion(name);
    setExclusions((l) => (l.includes(name) ? l : [...l, name]));
    setExclusion("");
    setMsg("App excluded.");
  };

  const saveDisplayName = async () => {
    const name = displayName.trim();
    // Empty = revert to the system (computer) name.
    await setSetting("display_name", name, "string");
    setMsg(name ? `Display name set to “${name}”.` : "Using the system name.");
  };

  const saveUpdateRepo = async () => {
    await setSetting("update_owner", updateOwner.trim(), "string");
    await setSetting("update_repo", updateRepo.trim(), "string");
    setMsg("Update feed saved.");
  };

  const checkForUpdate = async () => {
    setUpdateBusy(true);
    const info = await checkForUpdates(updateOwner.trim(), updateRepo.trim());
    setUpdateBusy(false);
    if (!info) {
      setUpdateInfo(null);
      setUpdateMsg("Could not reach the update feed. Check the owner/repo.");
      return;
    }
    setUpdateInfo(info);
    setUpdateMsg(
      info.has_update
        ? `Version ${info.latest} is available (you have ${info.installed}).`
        : `You’re up to date (${info.installed}).`,
    );
  };

  const doUpdate = async () => {
    setUpdateMsg("Downloading & installing… the app will restart automatically.");
    const ok = await applyUpdate(updateOwner.trim(), updateRepo.trim());
    if (!ok) {
      setUpdateMsg("Update failed during download. Try again.");
      return;
    }
    setTimeout(() => void exitApp(), 800);
  };

  const doBackup = async () => {
    const target = backupPath.trim() || `${window.console ? "" : ""}`;
    if (!target) return;
    const ok = await backupDb(target);
    setMsg(ok ? `Backup saved to ${target}` : "Backup failed.");
  };

  return (
    <div className="mx-auto max-w-3xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-(--text-secondary)">Preferences, privacy & maintenance</p>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Monitoring</CardTitle>
            <CardDescription>Pause recording or tweak thresholds</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Pause monitoring</p>
              <p className="text-xs text-(--text-tertiary)">Temporarily stop recording activity</p>
            </div>
            <button
              type="button"
              onClick={togglePaused}
              className={`relative h-6 w-11 rounded-full transition-colors ${paused ? "bg-(--cat-entertainment)" : "bg-(--cat-productivity)"}`}
            >
              <span
                className={`absolute top-0.5 size-5 rounded-full bg-white shadow transition-all ${paused ? "left-[22px]" : "left-0.5"}`}
              />
            </button>
          </div>

          <div className="flex items-end gap-3">
            <label className="flex-1 space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Idle threshold (seconds — 1200 = 20 min)</span>
              <input
                type="number"
                value={idle}
                onChange={(e) => setIdle(e.target.value)}
                className="w-full rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
              />
            </label>
            <Button variant="secondary" onClick={saveIdle}>Save</Button>
          </div>

          <div className="flex items-end gap-3">
            <label className="flex-1 space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Daily screen-time goal (hours)</span>
              <input
                type="number"
                min={1}
                max={24}
                value={goalHours}
                onChange={(e) => setGoalHours(e.target.value)}
                className="w-full rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
              />
            </label>
            <Button variant="secondary" onClick={saveGoal}>Save</Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Device / display name</CardTitle>
            <CardDescription>Shown on the dashboard greeting — blank uses your system name</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="flex items-end gap-3">
          <label className="flex-1 space-y-1.5">
            <span className="text-xs font-medium text-(--text-secondary)">Display name</span>
            <input
              type="text"
              placeholder="Your PC’s name"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && saveDisplayName()}
              className="w-full rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
            />
          </label>
          <Button variant="secondary" onClick={saveDisplayName}>Save</Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Updates</CardTitle>
            <CardDescription>Check for &amp; install the latest release from GitHub Releases</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">GitHub owner</span>
              <input
                type="text"
                placeholder="e.g. your-username"
                value={updateOwner}
                onChange={(e) => setUpdateOwner(e.target.value)}
                className="w-full rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
              />
            </label>
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Repository</span>
              <input
                type="text"
                placeholder="e.g. vigil"
                value={updateRepo}
                onChange={(e) => setUpdateRepo(e.target.value)}
                className="w-full rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
              />
            </label>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="secondary" onClick={saveUpdateRepo}>Save feed</Button>
            <Button onClick={checkForUpdate} disabled={updateBusy}>
              {updateBusy ? "Checking…" : "Check for updates"}
            </Button>
            {updateInfo?.has_update && (
              <Button onClick={doUpdate}>Download &amp; update</Button>
            )}
          </div>
          {updateMsg && <p className="text-xs text-(--text-tertiary)">{updateMsg}</p>}
          {updateInfo && (updateInfo.assets?.length ?? 0) > 0 && (
            <p className="break-words text-[11px] text-(--text-tertiary)">
              Release assets: {updateInfo.assets.join(", ")}
            </p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Dashboard logging</CardTitle>
            <CardDescription>Write UI activity &amp; errors to dashboard.log</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Enable logging</p>
              <p className="text-xs text-(--text-tertiary)">
                Logs go to %LOCALAPPDATA%\Vigil\dashboard.log and the console.
              </p>
            </div>
            <button
              type="button"
              onClick={() => {
                const next = !loggingOn;
                setLoggingOn(next);
                setLogEnabled(next);
                void setSetting("dashboard_logging", String(next), "boolean");
                setMsg(next ? "Logging enabled." : "Logging disabled.");
              }}
              className={`relative h-6 w-11 rounded-full transition-colors ${loggingOn ? "bg-(--cat-productivity)" : "bg-(--cat-system)"}`}
            >
              <span
                className={`absolute top-0.5 size-5 rounded-full bg-white shadow transition-all ${loggingOn ? "left-[22px]" : "left-0.5"}`}
              />
            </button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Exclusions</CardTitle>
            <CardDescription>Apps that will never be recorded</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="e.g. KeePass.exe"
              value={exclusion}
              onChange={(e) => setExclusion(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addExclusionFn()}
              className="flex-1 rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
            />
            <Button variant="secondary" onClick={addExclusionFn}>Add</Button>
          </div>
          {exclusions.length > 0 && (
            <ul className="flex flex-wrap gap-2">
              {exclusions.map((name) => (
                <li key={name} className="rounded-full bg-(--surface-2) px-3 py-1 text-xs text-(--text-secondary) ring-1 ring-(--border-subdued)">
                  {name}
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Report export location</CardTitle>
            <CardDescription>Where exported CSV reports go by default</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="flex gap-2">
          <input
            type="text"
            placeholder="e.g. C:\\Users\\you\\Documents\\Vigil Reports"
            value={exportDir}
            onChange={(e) => setExportDir(e.target.value)}
            className="flex-1 rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
          />
          <Button
            variant="secondary"
            onClick={async () => {
              await setSetting("export_dir", exportDir.trim(), "string");
              setMsg(exportDir.trim() ? `Export folder set to ${exportDir.trim()}` : "Export folder cleared.");
            }}
          >
            Save
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Data retention</CardTitle>
            <CardDescription>Keep raw data for N days, then auto-purge (default 90)</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="flex gap-2">
          <input
            type="number"
            min={1}
            max={3650}
            value={retention}
            onChange={(e) => setRetentionState(e.target.value)}
            className="w-32 rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
          />
          <Button variant="secondary" onClick={saveRetention}>Save &amp; purge now</Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Backup</CardTitle>
            <CardDescription>Snapshot the database (SQLite-safe)</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="flex gap-2">
          <input
            type="text"
            placeholder="C:\\Users\\you\\Documents\\vigil_backup.db"
            value={backupPath}
            onChange={(e) => setBackupPath(e.target.value)}
            className="flex-1 rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40"
          />
          <Button variant="secondary" onClick={doBackup}>Back up</Button>
        </CardContent>
      </Card>

      {msg && <p className="text-xs text-(--text-tertiary)">{msg}</p>}
    </div>
  );
}
