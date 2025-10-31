import { useSettings } from "../../hooks/useSettings";
import { GeneralSettings } from "../settings/GeneralSettings";
import { AdvancedSettings } from "../settings/AdvancedSettings";
import { AboutSettings } from "../settings/AboutSettings";
import { DebugSettings } from "../settings/DebugSettings";
import { useState } from "react";

type SettingsTab = "general" | "advanced" | "debug" | "about";

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");
  const { settings, updateSetting } = useSettings();

  // Show loading state while settings are being fetched
  if (!settings) {
    return (
      <div className="flex-1 flex items-center justify-center bg-gray-950">
        <div className="text-gray-400">Loading settings...</div>
      </div>
    );
  }

  const tabs = [
    { id: "general" as const, label: "General" },
    { id: "advanced" as const, label: "Advanced" },
    { id: "debug" as const, label: "Debug" },
    { id: "about" as const, label: "About" },
  ];

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h1 className="text-2xl font-semibold text-gray-100">Settings</h1>
        <p className="text-sm text-gray-400 mt-1">Configure Ultra Whisper preferences</p>
      </div>

      {/* Tab Navigation */}
      <div className="px-6 py-3 border-b border-gray-800">
        <div className="flex gap-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-4 py-2 text-sm font-medium rounded-md transition-colors ${
                activeTab === tab.id
                  ? "bg-green-500/10 text-green-500"
                  : "text-gray-400 hover:text-gray-200 hover:bg-gray-800/50"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="p-6 max-w-4xl mx-auto">
          {activeTab === "general" && <GeneralSettings />}
          {activeTab === "advanced" && <AdvancedSettings />}
          {activeTab === "debug" && <DebugSettings />}
          {activeTab === "about" && <AboutSettings />}
        </div>
      </div>

      {/* Debug Mode Indicator */}
      {settings.debug_mode && (
        <div className="px-6 py-2 bg-amber-500/10 border-t border-amber-500/20">
          <p className="text-xs text-amber-500">
            Debug mode is enabled. Press Ctrl+Shift+D (Cmd+Shift+D on macOS) to toggle.
          </p>
        </div>
      )}
    </div>
  );
}