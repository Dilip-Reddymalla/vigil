# Vigil Reports (Phase 6)

## CSV export

The service exposes `reports.export_csv` over IPC:

```json
{ "jsonrpc": "2.0", "method": "reports.export_csv",
  "params": { "local_date": "2026-11-13", "target_path": "C:\\reports\\2026-11-13.csv" },
  "id": 1 }
```

Writes every `app_sessions` row for that local day as CSV
(`session_uuid,process_name,category,is_browser,start_time,end_time,duration_ms,status,active_ratio`).

## Planned

- PDF export (client-side render of the same data).
- Weekly/monthly rollups from `daily_summaries`.
- NSIS installer that bundles both binaries, registers Task Scheduler auto-start,
  and a read-only dashboard build.
