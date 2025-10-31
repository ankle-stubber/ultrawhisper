import React from "react";
import { MicrophoneSelector } from "./MicrophoneSelector";
import { LanguageSelector } from "./LanguageSelector";
import { ConfigCard } from "../shared/ConfigCard";
import { OutputDeviceSelector } from "./OutputDeviceSelector";
import { PushToTalk } from "./PushToTalk";
import { AudioFeedback } from "./AudioFeedback";
import { useSettings } from "../../hooks/useSettings";
import { VolumeSlider } from "./VolumeSlider";

export const GeneralSettings: React.FC = () => {
  const { audioFeedbackEnabled } = useSettings();
  return (
    <div className="space-y-6">
      <ConfigCard title="General">
        <LanguageSelector descriptionMode="tooltip" grouped={true} />
        <PushToTalk descriptionMode="tooltip" grouped={true} />
      </ConfigCard>
      <ConfigCard title="Sound">
        <MicrophoneSelector descriptionMode="tooltip" grouped={true} />
        <AudioFeedback descriptionMode="tooltip" grouped={true} />
        <OutputDeviceSelector
          descriptionMode="tooltip"
          grouped={true}
          disabled={!audioFeedbackEnabled}
        />
        <VolumeSlider disabled={!audioFeedbackEnabled} />
      </ConfigCard>
    </div>
  );
};
