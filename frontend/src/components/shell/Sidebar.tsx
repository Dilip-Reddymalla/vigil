import { NavLink } from "react-router-dom";
import { cn } from "@/lib/utils";
import { VigilLogo } from "@/components/ui/logo";
import { Icon } from "@/components/ui/icon";

const NAV = [
  { to: "/", label: "Dashboard", icon: "dashboard" },
  { to: "/applications", label: "Applications", icon: "applications" },
  { to: "/websites", label: "Websites", icon: "websites" },
  { to: "/network", label: "Network", icon: "network" },
  { to: "/timeline", label: "Timeline", icon: "timeline" },
  { to: "/reports", label: "Reports", icon: "reports" },
  { to: "/settings", label: "Settings", icon: "settings" },
];

export function Sidebar({
  collapsed,
  onToggle,
}: {
  collapsed: boolean;
  onToggle: () => void;
}) {
  return (
    <aside
      className={cn(
        "flex flex-col border-r border-(--border-subdued) bg-(--surface) transition-[width] duration-200 ease-out",
        collapsed ? "w-16" : "w-[220px]",
      )}
    >
      <div className="flex h-14 items-center gap-2.5 px-4">
        <VigilLogo size={34} />
        {!collapsed && (
          <div className="leading-tight">
            <p className="text-sm font-semibold tracking-tight">Vigil</p>
            <p className="text-[10px] text-(--text-tertiary)">Digital wellbeing</p>
          </div>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-2 py-2">
        {NAV.map(({ to, label, icon }) => (
          <NavLink
            key={to}
            to={to}
            title={label}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-(--text-secondary) transition-colors hover:bg-black/5 dark:hover:bg-white/10",
                isActive &&
                  "bg-(--cat-development)/10 text-(--cat-development) hover:bg-(--cat-development)/10",
              )
            }
          >
            <Icon name={icon} size={20} />
            {!collapsed && <span>{label}</span>}
          </NavLink>
        ))}
      </nav>

      <button
        type="button"
        onClick={onToggle}
        className="m-2 flex items-center gap-3 rounded-lg px-3 py-2 text-sm text-(--text-secondary) hover:bg-black/5 dark:hover:bg-white/10"
      >
        <Icon name={collapsed ? "sidebar-expand" : "sidebar-collapse"} size={20} />
        {!collapsed && <span>Collapse</span>}
      </button>
    </aside>
  );
}
