import { useState } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "@/components/shell/Sidebar";
import { Header } from "@/components/shell/Header";
import { ConnectionBanner } from "@/components/shell/ConnectionBanner";
import { ErrorBoundary } from "@/components/shell/ErrorBoundary";
import { cn } from "@/lib/utils";

/**
 * AppShell — the frameless window shell.
 * In the real Tauri build the window is frameless with `data-tauri-drag-region`
 * on the Header; here we render the layout for browser development.
 */
export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-(--background) text-(--text-primary)">
      <Sidebar collapsed={collapsed} onToggle={() => setCollapsed((c) => !c)} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Header />
        <main
          className={cn(
            "flex-1 overflow-y-auto px-6 py-6 transition-all duration-200 ease-out",
          )}
        >
          <ConnectionBanner />
          <ErrorBoundary>
            <Outlet />
          </ErrorBoundary>
        </main>
      </div>
    </div>
  );
}
