import React from "react";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { SettingContainer } from "../ui/SettingContainer";
import { Dropdown } from "../ui/Dropdown";
import { useSettings } from "../../hooks/useSettings";
import type { CleaningSettings } from "../../lib/types";

interface TranscriptionCleaningProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const TranscriptionCleaning: React.FC<TranscriptionCleaningProps> = React.memo(({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { settings, updateSetting, isUpdating } = useSettings();

  const cleaningEnabled = settings?.cleaning?.enabled || false;
  const cleaningProfile = settings?.cleaning?.profile || "basic";

  const handleToggle = (enabled: boolean) => {
    // Initialize cleaning settings if missing, but DO NOT set rules here.
    // Leaving rules undefined lets the backend apply its default rule set.
    const currentCleaning: Partial<CleaningSettings> = settings?.cleaning ?? {
      enabled: false,
      profile: "basic",
    };

    updateSetting("cleaning", {
      ...currentCleaning,
      enabled,
    } as CleaningSettings);
  };

  return (
    <>
      <ToggleSwitch
        checked={cleaningEnabled}
        onChange={handleToggle}
        isUpdating={isUpdating("cleaning")}
        label="Clean Transcription Text"
        description="Apply post-processing to clean up spacing and punctuation in transcripts."
        descriptionMode={descriptionMode}
        grouped={grouped}
      />

      {cleaningEnabled && (
        <SettingContainer
          title="Cleaning Profile"
          description="Select the rule preset. Disfluency removes fillers like ‘uh/um’."
          descriptionMode={descriptionMode}
          grouped
        >
          <div className="w-60">
            <Dropdown
              options={[
                { value: "basic", label: "Basic (Spacing & Punctuation)" },
                { value: "disfluency", label: "Disfluency (Remove uh/um)" },
              ]}
              selectedValue={cleaningProfile}
              disabled={isUpdating("cleaning")}
              onSelect={(value) => {
                // Ask backend to repopulate rules for the new profile by sending empty rules
                const current = settings?.cleaning ?? { enabled: true, profile: "basic" };
                updateSetting("cleaning", {
                  ...current,
                  profile: value,
                  rules: [],
                } as any);
              }}
            />
          </div>
        </SettingContainer>
      )}
    </>
  );
});
