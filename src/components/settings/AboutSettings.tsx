import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SettingsGroup } from "../ui/SettingsGroup";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";

export const AboutSettings: React.FC = () => {
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  const handleDonateClick = async () => {
    try {
      await openUrl("https://handy.computer/donate");
    } catch (error) {
      console.error("Failed to open donate link:", error);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title="About">
        <SettingContainer
          title="Version"
          description="Current version of UltraWhisper"
          grouped={true}
        >
          <span className="text-sm font-mono">v{version}</span>
        </SettingContainer>

        <SettingContainer
          title="Source Code"
          description="View source code and contribute"
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() =>
              openUrl("https://github.com/ankle-stubber/ultrawhisper")
            }
          >
            UltraWhisper on GitHub
          </Button>
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title="Acknowledgments">
        <SettingContainer
          title="Handy"
          description="UltraWhisper is built on the foundation of Handy"
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            UltraWhisper started with a fork of Handy, which is a great
            open-source project that provides a robust and user-friendly
            platform for speech recognition and transcription. Check the project
            out and show them some love!
          </div>

          <Button
            variant="primary"
            size="md"
            onClick={handleDonateClick}
            className="mt-2"
          >
            Donate to Handy
          </Button>
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl("https://github.com/cjpais/handy")}
          >
            View Original Handy
          </Button>
        </SettingContainer>

        <SettingContainer
          title="Whisper.cpp"
          description="High-performance inference of
          OpenAI's Whisper automatic speech recognition model"
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            UltraWhisper uses Whisper.cpp for fast, local speech-to-text
            processing. Thanks to the amazing work by Georgi Gerganov and
            contributors.
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};
