import React, { useState, useEffect } from "react";
import { SettingsGroup } from "../ui/SettingsGroup";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";
import { BareToggle } from "../ui/BareToggle";

interface ActiveWindowConfig {
  type: "active_window";
  paste_method: "ctrl_v" | "direct";
  preserve_clipboard: boolean;
}

interface ActiveWindowSetupProps {
  config: ActiveWindowConfig;
  onSave: (config: ActiveWindowConfig) => Promise<void>;
}

export default function ActiveWindowSetup({ config, onSave }: ActiveWindowSetupProps) {
  // Form state
  const [pasteMethod, setPasteMethod] = useState<"ctrl_v" | "direct">(config.paste_method);
  const [preserveClipboard, setPreserveClipboard] = useState(config.preserve_clipboard);

  // Original config for dirty tracking
  const [originalConfig, setOriginalConfig] = useState({
    paste_method: config.paste_method,
    preserve_clipboard: config.preserve_clipboard,
  });

  // UI state
  const [isSaving, setIsSaving] = useState(false);

  // Update form when config prop changes
  useEffect(() => {
    setPasteMethod(config.paste_method);
    setPreserveClipboard(config.preserve_clipboard);
    setOriginalConfig({
      paste_method: config.paste_method,
      preserve_clipboard: config.preserve_clipboard,
    });
  }, [config]);

  // Dirty detection
  const isDirty =
    pasteMethod !== originalConfig.paste_method ||
    preserveClipboard !== originalConfig.preserve_clipboard;

  // Handlers
  const handleSave = async () => {
    setIsSaving(true);
    try {
      await onSave({
        type: "active_window",
        paste_method: pasteMethod,
        preserve_clipboard: preserveClipboard,
      });
      // Reset baseline after successful save
      setOriginalConfig({
        paste_method: pasteMethod,
        preserve_clipboard: preserveClipboard,
      });
    } catch (error) {
      // Error toast handled by parent
      console.error("Save error:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRevert = () => {
    setPasteMethod(originalConfig.paste_method);
    setPreserveClipboard(originalConfig.preserve_clipboard);
  };

  return (
    <div className="w-full max-w-2xl space-y-4">
      <SettingsGroup title="ACTIVE WINDOW CONFIGURATION" description="Configure how text is pasted to the active application">
        <SettingContainer
          title="Paste Method"
          description="How transcribed text should be inserted into the active window"
          layout="stacked"
          descriptionMode="inline"
          grouped
        >
          <select
            value={pasteMethod}
            onChange={(e) => setPasteMethod(e.target.value as "ctrl_v" | "direct")}
            disabled={isSaving}
            className="w-full px-3 py-2 bg-white dark:bg-dark-bg border border-mid-gray/30 rounded-md text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-logo-primary transition-all"
          >
            <option value="ctrl_v">Ctrl+V / Cmd+V (Standard)</option>
            <option value="direct">Direct Paste</option>
          </select>
        </SettingContainer>

        <SettingContainer
          title="Preserve Clipboard"
          description="Keep clipboard contents after pasting transcription"
          layout="stacked"
          descriptionMode="inline"
          grouped
        >
          <div className="flex items-center">
            <BareToggle
              checked={preserveClipboard}
              onChange={setPreserveClipboard}
              disabled={isSaving}
            />
          </div>
        </SettingContainer>
      </SettingsGroup>

      <div className="flex gap-2 justify-end">
        <Button
          variant="secondary"
          onClick={handleRevert}
          disabled={!isDirty || isSaving}
        >
          Revert
        </Button>
        <Button
          variant="primary"
          onClick={handleSave}
          disabled={!isDirty || isSaving}
        >
          {isSaving ? "Saving..." : "Save"}
        </Button>
      </div>
    </div>
  );
}
