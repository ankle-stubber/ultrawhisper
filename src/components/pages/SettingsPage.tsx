import { useSettings } from "../../hooks/useSettings";
import { GeneralSettings } from "../settings/GeneralSettings";
import { AppearanceSettings } from "../settings/AppearanceSettings";
import { AdvancedSettings } from "../settings/AdvancedSettings";
import { AboutSettings } from "../settings/AboutSettings";
import { DebugSettings } from "../settings/DebugSettings";
import { PageHeader } from "../shared";
import { useState } from "react";

type SettingsTab = "general" | "appearance" | "advanced" | "debug" | "about";

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");
  const { settings, updateSetting } = useSettings();

  // Show loading state while settings are being fetched
  if (!settings) {
    return (
      <div className="flex-1 flex items-center justify-center uw-bg-surface">
        <div className="uw-text-secondary">Loading settings...</div>
      </div>
    );
  }

  const tabs = [
    { id: "general" as const, label: "General" },
    { id: "appearance" as const, label: "Appearance" },
    { id: "advanced" as const, label: "Advanced" },
    { id: "debug" as const, label: "Debug" },
    { id: "about" as const, label: "About" },
  ];

  return (
    <div className="flex-1 flex flex-col h-full uw-bg-surface">
      {/* Header */}
      <PageHeader
        title="Settings"
        subtitle="Configure Ultra Whisper preferences"
      />

      {/* Tab Navigation */}
      <div className="px-6 py-3 border-b uw-border-default">
        <div className="flex gap-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-4 py-2 text-sm font-medium rounded-md transition-colors ${
                activeTab === tab.id
                  ? "uw-bg-primary-dim uw-text-accent"
                  : "uw-text-secondary hover:uw-text-primary hover:uw-bg-card"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden uw-scroll">
        <div className="p-6">
          {activeTab === "general" && <GeneralSettings />}
          {activeTab === "appearance" && <AppearanceSettings />}
          {activeTab === "advanced" && <AdvancedSettings />}
          {activeTab === "debug" && <DebugSettings />}
          {activeTab === "about" && <AboutSettings />}
        </div>
      </div>

      {/* Debug Mode Indicator */}
      {settings.debug_mode && (
        <div className="px-6 py-2 uw-bg-warning-dim border-t uw-border-warning">
          <p className="text-xs uw-text-warning">
            Debug mode is enabled. Press Ctrl+Shift+D (Cmd+Shift+D on macOS) to toggle.
          </p>
        </div>
      )}
    </div>
  );
}
