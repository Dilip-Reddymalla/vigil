import { useEffect, useState } from "react";
import { exportReport, getSetting, openReportFolder, pickFolder, today } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type Granularity = "daily" | "weekly" | "monthly";
type Format = "csv" | "pdf";

export function ReportsView() {
  const [granularity, setGranularity] = useState<Granularity>("daily");
  const [format, setFormat] = useState<Format>("csv");
  const [date, setDate] = useState(today());
  const [path, setPath] = useState("");
  const [exportDir, setExportDir] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    getSetting("export_dir").then((v) => v && setExportDir(v));
  }, []);

  const onExport = async () => {
    setStatus("Exporting…");
    // Blank manual path → use the configured export folder (or the service default).
    const ext = format;
    const target =
      path.trim() || (exportDir ? `${exportDir}\\${date}-report.${ext}` : "");
    const written = await exportReport(date, target, granularity, format);
    setStatus(written ? `Exported to ${written}` : "Export failed.");
  };

  const onPickFolder = async () => {
    const dir = await pickFolder();
    if (dir) {
      const p = `${dir}\\${date}-report.${format}`;
      setPath(p);
      setMsg(`Folder selected: ${p}`);
    }
  };

  const selectCls =
    "rounded-lg border border-(--border-subdued) bg-(--surface-2) px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-(--accent)/40";

  return (
    <div className="mx-auto max-w-3xl space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Reports</h1>
        <p className="text-sm text-(--text-secondary)">Export your data</p>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Report export</CardTitle>
            <CardDescription>CSV or PDF — daily, weekly, or monthly</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Granularity</span>
              <select
                value={granularity}
                onChange={(e) => setGranularity(e.target.value as Granularity)}
                className={selectCls + " w-full"}
              >
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="monthly">Monthly</option>
              </select>
            </label>
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Format</span>
              <select value={format} onChange={(e) => setFormat(e.target.value as Format)} className={selectCls + " w-full"}>
                <option value="csv">CSV</option>
                <option value="pdf">PDF</option>
              </select>
            </label>
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Anchor date</span>
              <input
                type="date"
                value={date}
                onChange={(e) => setDate(e.target.value)}
                className={selectCls + " w-full"}
              />
            </label>
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-(--text-secondary)">Target path (optional)</span>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={path}
                  placeholder={exportDir ? `default: ${exportDir}` : "leave blank → %LOCALAPPDATA%\\Vigil\\reports"}
                  onChange={(e) => setPath(e.target.value)}
                  className={selectCls + " w-full"}
                />
                <Button variant="secondary" type="button" onClick={onPickFolder}>
                  Browse…
                </Button>
              </div>
            </label>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button onClick={onExport}>Export {format.toUpperCase()}</Button>
            <Button variant="secondary" onClick={() => openReportFolder()}>
              Open reports folder
            </Button>
            {status && <span className="text-xs text-(--text-secondary)">{status}</span>}
            {msg && <span className="text-xs text-(--text-tertiary)">{msg}</span>}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
