import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Toaster } from "sonner";
import "./App.css";
import AccessibilityPermissions from "./components/AccessibilityPermissions";
import Footer from "./components/footer";
import UltraWhisperTextLogo from "./components/icons/UltraWhisperTextLogo";
import Onboarding from "./components/onboarding";
import { ThreePanelLayout } from "./components/layout/ThreePanelLayout";
import { WorkflowsList } from "./components/workflows/WorkflowsList";
import { WorkflowDetail } from "./components/workflows/WorkflowDetail";
import { DestinationsList } from "./components/destinations/DestinationsList";
import { DestinationDetail } from "./components/destinations/DestinationDetail";
import { ModelsList } from "./components/models/ModelsList";
import { ModelDetail } from "./components/models/ModelDetail";
import { HistoryList } from "./components/history/HistoryList";
import { HistoryDetail } from "./components/history/HistoryDetail";
import { LogsList } from "./components/logs/LogsList";
import { LogsDetail } from "./components/logs/LogsDetail";
import { Category } from "./lib/types";
import { useSettings } from "./hooks/useSettings";
import { useNavigationStore } from "./stores/navigationStore";

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const { activeCategory, setActiveCategory } = useNavigationStore();
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const { settings, updateSetting } = useSettings();

  useEffect(() => {
    checkOnboardingStatus();
  }, []);

  // Handle keyboard shortcuts for debug mode toggle
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Check for Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
      const isDebugShortcut =
        event.shiftKey &&
        event.key.toLowerCase() === "d" &&
        (event.ctrlKey || event.metaKey);

      if (isDebugShortcut) {
        event.preventDefault();
        const currentDebugMode = settings?.debug_mode ?? false;
        updateSetting("debug_mode", !currentDebugMode);
      }
    };

    // Add event listener when component mounts
    document.addEventListener("keydown", handleKeyDown);

    // Cleanup event listener when component unmounts
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [settings?.debug_mode, updateSetting]);

  const checkOnboardingStatus = async () => {
    try {
      // Always check if they have any models available
      const modelsAvailable: boolean = await invoke("has_any_models_available");
      setShowOnboarding(!modelsAvailable);
    } catch (error) {
      console.error("Failed to check onboarding status:", error);
      setShowOnboarding(true);
    }
  };

  const handleModelSelected = () => {
    // Transition to main app - user has started a download
    setShowOnboarding(false);
  };

  if (showOnboarding) {
    return (
      <div className="h-screen flex flex-col">
        <div className="flex-1 flex flex-col items-center justify-center p-4 gap-2">
          <UltraWhisperTextLogo width={200} />
          <Onboarding onModelSelected={handleModelSelected} />
        </div>
      </div>
    );
  }

  // Auto-select first item when category changes
  useEffect(() => {
    switch (activeCategory) {
      case "workflows":
        setSelectedItemId("batch-processing");
        break;
      case "destinations":
        setSelectedItemId("telegram-default");
        break;
      case "models":
        setSelectedItemId("model-management");
        break;
      case "history":
        setSelectedItemId("all-transcriptions");
        break;
      case "logs":
        setSelectedItemId("application-logs");
        break;
    }
  }, [activeCategory]);

  // Render items panel based on active category
  const renderItemsPanel = () => {
    switch (activeCategory) {
      case "workflows":
        return (
          <WorkflowsList
            selectedId={selectedItemId}
            onSelect={setSelectedItemId}
          />
        );
      case "destinations":
        return (
          <DestinationsList
            selectedId={selectedItemId}
            onSelect={setSelectedItemId}
          />
        );
      case "models":
        return (
          <ModelsList
            selectedId={selectedItemId}
            onSelect={setSelectedItemId}
          />
        );
      case "history":
        return (
          <HistoryList
            selectedId={selectedItemId}
            onSelect={setSelectedItemId}
          />
        );
      case "logs":
        return (
          <LogsList
            selectedId={selectedItemId}
            onSelect={setSelectedItemId}
          />
        );
    }
  };

  // Render detail panel based on active category
  const renderDetailPanel = () => {
    return (
      <div className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0">
        <div className="flex-1 overflow-y-auto w-full">
          <div className="flex flex-col p-4 gap-4 w-full">
            <AccessibilityPermissions />
            {activeCategory === "workflows" && (
              <WorkflowDetail workflowId={selectedItemId} />
            )}
            {activeCategory === "destinations" && (
              <DestinationDetail destinationId={selectedItemId} />
            )}
            {activeCategory === "models" && (
              <ModelDetail modelId={selectedItemId} />
            )}
            {activeCategory === "history" && (
              <HistoryDetail historyId={selectedItemId} />
            )}
            {activeCategory === "logs" && (
              <LogsDetail logId={selectedItemId} />
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="h-screen flex flex-col">
      <Toaster />
      {/* Main content area that takes remaining space */}
      <div className="flex-1 flex overflow-hidden">
        <ThreePanelLayout
          activeCategory={activeCategory}
          onCategoryChange={setActiveCategory}
          selectedItemId={selectedItemId}
          onItemSelect={setSelectedItemId}
          itemsPanel={renderItemsPanel()}
          detailPanel={renderDetailPanel()}
        />
      </div>
      {/* Fixed footer at bottom */}
      <Footer />
    </div>
  );
}

export default App;
