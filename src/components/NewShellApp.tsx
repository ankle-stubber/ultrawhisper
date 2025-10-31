import { useState, useEffect } from "react";
import { Toaster } from "sonner";
import { ErrorBoundary } from "./ErrorBoundary";
import { Sidebar } from "./navigation/Sidebar";
import { SystemMonitorPage } from "./pages/SystemMonitorPage";
import { WorkflowsPage } from "./pages/WorkflowsPage";
import { DestinationsPage } from "./pages/DestinationsPage";
import { ModelsPage } from "./pages/ModelsPage";
import { HistoryPage } from "./pages/HistoryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { LiveTranscriptionPage } from "./pages/LiveTranscriptionPage";
import { useThemeStore, initializeTheme } from "../stores/themeStore";

type Page = "monitor" | "workflows" | "destinations" | "models" | "history" | "settings" | "live";

export function NewShellApp() {
  const [currentPage, setCurrentPage] = useState<Page>("monitor");
  const themeClassName = useThemeStore((state) => state.getThemeClassName());

  // Initialize theme on mount
  useEffect(() => {
    initializeTheme();
  }, []);

  const renderPage = () => {
    switch (currentPage) {
      case "monitor":
        return <SystemMonitorPage />;
      case "workflows":
        return <WorkflowsPage />;
      case "destinations":
        return <DestinationsPage />;
      case "models":
        return <ModelsPage />;
      case "history":
        return <HistoryPage />;
      case "settings":
        return <SettingsPage />;
      case "live":
        return <LiveTranscriptionPage />;
      default:
        return <SystemMonitorPage />;
    }
  };

  return (
    <ErrorBoundary>
      <div className={`flex h-screen uw-bg-surface ${themeClassName}`}>
        <Toaster position="bottom-right" />

        {/* Sidebar Navigation */}
        <Sidebar
          currentPage={currentPage}
          onNavigate={setCurrentPage}
        />

        {/* Main Content Area */}
        <main className="flex-1 flex overflow-hidden">
          {renderPage()}
        </main>
      </div>
    </ErrorBoundary>
  );
}