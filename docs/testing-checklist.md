# Vigil Testing Checklist

Adapted from the design review. This is the acceptance checklist applied as the
phases land.

## Tracking (Phase 1–2)
- [ ] Rapidly switch between applications → verify session boundaries (no overlap, no lost time).
- [ ] Leave idle beyond the configured threshold → verify tracking pauses/resumes correctly.
- [ ] Lock/unlock → compare lock time with idle time.
- [ ] Sleep/resume → no phantom active session.
- [ ] Cross local midnight while active → session splits on the correct `local_date`.
- [ ] Force-terminate service and restart → orphaned session recovered via heartbeat.

## Lifecycle & IPC (Phase 3)
- [ ] Close dashboard while monitoring continues; reopen → data consistent.
- [ ] Dashboard queries service status reliably; settings mutations succeed.
- [ ] Pipe access restricted to the current user SID.

## Browser & network (Phase 4)
- [ ] Chrome/Edge URL extraction verified.
- [ ] Wi-Fi/Ethernet disconnect/reconnect → network deltas remain non-negative.
- [ ] Incognito: time counted, URL not recorded.
- [ ] Excluded apps/domains not recorded; existing data preserved.

## UI (Phase 5)
- [ ] 60fps animations; responsive layout; clean exit (`app.exit(0)`, 0 MB when closed).
- [ ] Dark/light themes; every view has a designed empty state.

## Release (Phase 6)
- [ ] 24-hour profiling: service RAM ≤ 15 MB, CPU < 0.5%, DB growth < 500 KB.
- [ ] Install/upgrade/uninstall clean; retention purge and backup/restore verified.
