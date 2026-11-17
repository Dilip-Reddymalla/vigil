import { BrowserRouter, Route, Routes } from "react-router-dom";
import { AppShell } from "@/components/shell/AppShell";
import { DashboardView } from "@/views/DashboardView";
import { ApplicationsView } from "@/views/ApplicationsView";
import { WebsitesView } from "@/views/WebsitesView";
import { NetworkView } from "@/views/NetworkView";
import { TimelineView } from "@/views/TimelineView";
import { ReportsView } from "@/views/ReportsView";
import { SettingsView } from "@/views/SettingsView";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<DashboardView />} />
          <Route path="/applications" element={<ApplicationsView />} />
          <Route path="/websites" element={<WebsitesView />} />
          <Route path="/network" element={<NetworkView />} />
          <Route path="/timeline" element={<TimelineView />} />
          <Route path="/reports" element={<ReportsView />} />
          <Route path="/settings" element={<SettingsView />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
