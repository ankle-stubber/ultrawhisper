import React from "react";
import { ShowOverlay } from "./ShowOverlay";
import { TranslateToEnglish } from "./TranslateToEnglish";
import { ModelUnloadTimeoutSetting } from "./ModelUnloadTimeout";
import { CustomWords } from "./CustomWords";
import { TranscriptionCleaning } from "./TranscriptionCleaning";
import { ConfigCard } from "../shared/ConfigCard";
import { StartHidden } from "./StartHidden";
import { AutostartToggle } from "./AutostartToggle";
import BatchProcessingSettings from "./BatchProcessingSettings";
import { StreamingSettings } from "./StreamingSettings";
import { TelegramSetup } from "../destinations/TelegramSetup";

export const AdvancedSettings: React.FC = () => {
  return (
    <div className="space-y-6">
      <ConfigCard title="Advanced">
        <StartHidden descriptionMode="tooltip" grouped={true} />
        <AutostartToggle descriptionMode="tooltip" grouped={true} />
        <ShowOverlay descriptionMode="tooltip" grouped={true} />
        <TranslateToEnglish descriptionMode="tooltip" grouped={true} />
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped={true} />
        <CustomWords descriptionMode="tooltip" grouped />
        <TranscriptionCleaning descriptionMode="tooltip" grouped />
      </ConfigCard>

      {/* Streaming Settings */}
      <ConfigCard title="Streaming">
        <StreamingSettings descriptionMode="tooltip" grouped={true} />
      </ConfigCard>

      {/* Telegram Settings (Bundle 4 MVP - will move to Destinations panel in Bundle 6) */}
      <ConfigCard title="Telegram (MVP)">
        <TelegramSetup
          credentialId="telegram_default"
          descriptionMode="tooltip"
          grouped={true}
        />
      </ConfigCard>

      {/* Batch Processing as its own section */}
      <BatchProcessingSettings />
    </div>
  );
};
