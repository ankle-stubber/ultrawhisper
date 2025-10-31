import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import "./App.css";
import UltraWhisperTextLogo from "./components/icons/UltraWhisperTextLogo";
import Onboarding from "./components/onboarding";
import { LegacyApp } from "./components/LegacyApp";
import { NewShellApp } from "./components/NewShellApp";
import { useSettings } from "./hooks/useSettings";

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const { settings, updateSetting } = useSettings();

  // Check if new shell is enabled via environment variable
  const useNewShell = import.meta.env.VITE_USE_NEW_SHELL === "true";

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

  // Show onboarding if needed
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

  // Feature flag gate: render new or legacy shell
  if (useNewShell) {
    console.log("Using NEW shell (VITE_USE_NEW_SHELL=true)");
    return <NewShellApp />;
  } else {
    console.log("Using LEGACY shell (VITE_USE_NEW_SHELL=false or not set)");
    return <LegacyApp />;
  }
}

export default App;
