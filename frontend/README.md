# `frontend` — Vigil Dashboard UI

React 19 + TypeScript + Vite + Tailwind CSS 4 + shadcn/ui, following the Vigil
Apple/Fluent design system (`src/styles/globals.css`).

**Status: Phase 5 (scaffold).** The shell, view routing, and shared data hooks are
in place. The backend IPC wiring (`dashboard.get_summary` etc.) lands in Phase 5
when `src-tauri` becomes a real Tauri v2 app.

## Run (browser dev)

```powershell
cd frontend
npm install
npm run dev        # http://localhost:1420
```

## Run under Tauri (Phase 5)

```powershell
npm run tauri dev  # requires the src-tauri shell to be wired
```

## Structure

```
src/
├── styles/globals.css     # Tailwind 4 + Vigil design tokens (light/dark)
├── lib/utils.ts           # cn(), formatters
├── types/                 # shared API types (mirrors IPC protocol)
├── hooks/useDashboard.ts  # data-fetching hook (Tauri invoke)
├── components/
│   ├── ui/                # shadcn primitives (Button, …)
│   ├── shell/             # AppShell, Sidebar, Header
│   └── charts/            # (Phase 5) ActivityRing, HourlyTimeline
└── views/                 # Dashboard, Applications, Websites, Network,
                           #   Timeline, Reports, Settings
```

## Conventions

- Data fetching always via hooks wrapping Tauri IPC (`useQuery`, 30s staleTime).
- Every view must render a designed empty state.
- Charts: Recharts, dark-mode aware. Animations: Framer Motion (200ms ease-out).
