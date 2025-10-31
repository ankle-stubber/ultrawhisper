import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { SettingsGroup } from "../ui/SettingsGroup";
import { SettingContainer } from "../ui/SettingContainer";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

interface FileSystemConfig {
  type: "file_system";
  path: string;
  extension: string;
  filename_pattern: string;
}

interface FileSystemSetupProps {
  config: FileSystemConfig;
  onSave: (config: FileSystemConfig) => Promise<void>;
}

export default function FileSystemSetup({ config, onSave }: FileSystemSetupProps) {
  // Form state
  const [path, setPath] = useState(config.path);
  const [extension, setExtension] = useState(config.extension);
  const [filenamePattern, setFilenamePattern] = useState(config.filename_pattern);

  // Original config for dirty tracking
  const [originalConfig, setOriginalConfig] = useState({
    path: config.path,
    extension: config.extension,
    filename_pattern: config.filename_pattern,
  });

  // UI state
  const [isSaving, setIsSaving] = useState(false);
  const [extensionError, setExtensionError] = useState<string | null>(null);

  // Update form when config prop changes
  useEffect(() => {
    setPath(config.path);
    setExtension(config.extension);
    setFilenamePattern(config.filename_pattern);
    setOriginalConfig({
      path: config.path,
      extension: config.extension,
      filename_pattern: config.filename_pattern,
    });
  }, [config]);

  // Dirty detection
  const isDirty =
    path !== originalConfig.path ||
    extension !== originalConfig.extension ||
    filenamePattern !== originalConfig.filename_pattern;

  // Validation
  const isValid =
    path.trim().length > 0 &&
    extension.trim().length > 0 &&
    filenamePattern.trim().length > 0 &&
    !extensionError;

  // Handlers
  const handleExtensionChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const normalized = e.target.value.toLowerCase();
    setExtension(normalized);

    // Validate
    const trimmed = normalized.trim();
    if (!trimmed) {
      setExtensionError("Extension is required");
    } else if (!/^[a-z0-9]{2,10}$/.test(trimmed)) {
      setExtensionError("2-10 characters, letters and numbers only");
    } else {
      setExtensionError(null);
    }
  };

  const handleBrowse = async () => {
    try {
      const selectedPath = await invoke<string | null>("pick_directory");
      if (selectedPath) {
        setPath(selectedPath);
      }
    } catch (error) {
      console.error("Directory picker error:", error);
      toast.error(`Failed to pick directory: ${error}`);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await onSave({
        type: "file_system",
        path: path.trim(),
        extension: extension.trim(),
        filename_pattern: filenamePattern.trim(),
      });
      // Reset baseline after successful save
      setOriginalConfig({
        path: path.trim(),
        extension: extension.trim(),
        filename_pattern: filenamePattern.trim(),
      });
    } catch (error) {
      // Error toast handled by parent
      console.error("Save error:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRevert = () => {
    setPath(originalConfig.path);
    setExtension(originalConfig.extension);
    setFilenamePattern(originalConfig.filename_pattern);
    setExtensionError(null);
  };

  return (
    <div className="w-full space-y-4">
      <SettingsGroup title="FILE SYSTEM CONFIGURATION" description="Configure where and how transcriptions are saved">
        <SettingContainer
          title="Output Path"
          description="Directory where transcription files will be saved"
          layout="stacked"
          descriptionMode="inline"
          grouped
        >
          <div className="flex gap-2">
            <Input
              type="text"
              value={path}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPath(e.target.value)}
              placeholder="/Users/username/Documents/transcriptions"
              className="flex-1"
              disabled={isSaving}
            />
            <Button
              variant="secondary"
              onClick={handleBrowse}
              disabled={isSaving}
            >
              Browse…
            </Button>
          </div>
        </SettingContainer>

        <SettingContainer
          title="File Extension"
          description="Extension for saved files (e.g., md, txt, json)"
          layout="stacked"
          descriptionMode="inline"
          grouped
          error={extensionError || undefined}
        >
          <Input
            type="text"
            value={extension}
            onChange={handleExtensionChange}
            placeholder="md"
            className="w-full"
            disabled={isSaving}
          />
          {extensionError && (
            <p className="text-xs text-red-600 dark:text-red-400 mt-1">
              {extensionError}
            </p>
          )}
        </SettingContainer>

        <SettingContainer
          title="Filename Pattern"
          description="Pattern for generated filenames. Use {timestamp} for date/time."
          layout="stacked"
          descriptionMode="inline"
          grouped
        >
          <Input
            type="text"
            value={filenamePattern}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFilenamePattern(e.target.value)}
            placeholder="transcription_{timestamp}.md"
            className="w-full"
            disabled={isSaving}
          />
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
          disabled={!isDirty || !isValid || isSaving}
        >
          {isSaving ? "Saving..." : "Save"}
        </Button>
      </div>
    </div>
  );
}
