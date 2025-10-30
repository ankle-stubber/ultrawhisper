import React from "react";
import { ShowOverlay } from "./ShowOverlay";
import { TranslateToEnglish } from "./TranslateToEnglish";
import { ModelUnloadTimeoutSetting } from "./ModelUnloadTimeout";
import { CustomWords } from "./CustomWords";
import { SettingsGroup } from "../ui/SettingsGroup";
import { StartHidden } from "./StartHidden";
import { AutostartToggle } from "./AutostartToggle";
import BatchProcessingSettings from "./BatchProcessingSettings";
import { UseWorkflowEngine } from "./UseWorkflowEngine";
import { StreamingSettings } from "./StreamingSettings";
import { TelegramSetup } from "../destinations/TelegramSetup";

export const AdvancedSettings: React.FC = () => {
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title="Advanced">
        <StartHidden descriptionMode="tooltip" grouped={true} />
        <AutostartToggle descriptionMode="tooltip" grouped={true} />
        <ShowOverlay descriptionMode="tooltip" grouped={true} />
        <TranslateToEnglish descriptionMode="tooltip" grouped={true} />
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped={true} />
        <CustomWords descriptionMode="tooltip" grouped />
      </SettingsGroup>

      {/* Workflow Engine Settings */}
      <SettingsGroup title="Workflow Engine">
        <UseWorkflowEngine descriptionMode="tooltip" grouped={true} />
      </SettingsGroup>

      {/* Streaming Settings */}
      <SettingsGroup title="Streaming">
        <StreamingSettings descriptionMode="tooltip" grouped={true} />
      </SettingsGroup>

      {/* Telegram Settings (Bundle 4 MVP - will move to Destinations panel in Bundle 6) */}
      <SettingsGroup title="Telegram (MVP)">
        <TelegramSetup
          credentialId="telegram_default"
          descriptionMode="tooltip"
          grouped={true}
        />
      </SettingsGroup>

      {/* Batch Processing as its own section */}
      <BatchProcessingSettings />
    </div>
  );
};
