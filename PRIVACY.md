# Vigil Privacy Policy

**Effective date:** 2026-11-18

Vigil ("we", "our", "the application") is a local, privacy-first digital-activity
monitor for Windows 11. This policy explains what information the application
processes and how it is handled.

## 1. Data we process

Vigil runs entirely on your own computer and records your *activity* locally:

- **Screen-time & application usage** — app/process names, window titles, and how
  long each is in the foreground.
- **Website usage** — the domains you visit in a browser, page titles, and time
  spent per domain.
- **Project context** — project/workspace names parsed from IDE window titles.
- **Network usage** — cumulative interface-level bytes sent/received (not
  per-app or packet contents).
- **Battery status** — on-battery charge percentage and AC/battery state.
- **Derived analytics** — focus score, deep-work sessions, distraction index,
  weekly/monthly comparisons.

No keystrokes, page contents, network payloads, files, or audio are recorded.

## 2. Where data is stored

All data is stored **locally on your device** in a SQLite database at:

```
%LOCALAPPDATA%\Vigil\data.db
```

There is **no cloud, no telemetry, and no server-side storage**. The application
never uploads your activity data anywhere.

## 3. How data is used

Your data is used only to power the in-app dashboard and reports (screen time,
productivity, deep work, network totals, and trend comparisons). Nothing is sold,
shared, or transmitted.

## 4. Retention & deletion

- Raw data is retained for a configurable period (default 90 days) and then
  automatically purged.
- You can delete all data at any time by removing the
  `%LOCALAPPDATA%\Vigil` folder or uninstalling the application.
- You may also clear data via the Settings → Data retention controls.

## 5. Updates & network

The application checks for updates against a GitHub Releases feed you configure.
This check sends only the configured repository owner/repo and the installed
version to GitHub's public API. Update binaries are downloaded from that same
feed. No personal data is sent.

## 6. Third-party services

- **GitHub** — used solely for distributing release binaries and serving the
  update feed.
- **SignPath Foundation** — provides free code signing for the application's
  binaries (signatures only; no activity data).

Neither service receives your activity data.

## 7. Children

Vigil is not directed at children under 13, and we do not knowingly collect data
from them.

## 8. Changes to this policy

If this policy changes, the new version and effective date will be posted in this
repository.

## 9. Contact

For privacy questions, contact: reddymaladilip@gmail.com
