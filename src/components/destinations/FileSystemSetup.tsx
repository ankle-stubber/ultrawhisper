import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { open } from "@tauri-apps/plugin-dialog";
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
  onChange?: (field: string, value: any) => void; // For new destinations
  onSave?: (config: FileSystemConfig) => Promise<void>; // For existing destinations
  errors?: Record<string, string>; // Validation errors
  persistOnChange?: boolean; // Whether to auto-save on changes
}

export default function FileSystemSetup({
  config,
  onChange,
  onSave,
  errors = {},
  persistOnChange = true,
}: FileSystemSetupProps) {
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
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({});

  // Update form when config prop changes
  useEffect(() => {
    setPath(config.path);
    setExtension(config.extension);
    setFilenamePattern(config.filename_pattern);
    if (!persistOnChange) {
      // In creation mode, don't track original to avoid dirty state confusion
      return;
    }
    setOriginalConfig({
      path: config.path,
      extension: config.extension,
      filename_pattern: config.filename_pattern,
    });
  }, [config, persistOnChange]);

  // Dirty detection
  const isDirty =
    persistOnChange &&
    (path !== originalConfig.path ||
      extension !== originalConfig.extension ||
      filenamePattern !== originalConfig.filename_pattern);

  // Local validation
  const validateExtension = useCallback((value: string) => {
    const trimmed = value.trim();
    if (!trimmed) {
      return "Extension is required";
    } else if (!/^[a-z0-9]{1,10}$/i.test(trimmed)) {
      return "1-10 characters, letters and numbers only";
    }
    return null;
  }, []);

  // Handlers
  const handlePathChange = useCallback((value: string) => {
    setPath(value);
    if (onChange) {
      onChange("path", value);
    }
    // Clear path error
    setLocalErrors(prev => {
      const next = { ...prev };
      delete next.path;
      return next;
    });
  }, [onChange]);

  const handleExtensionChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.toLowerCase();
    setExtension(value);

    if (onChange) {
      onChange("extension", value);
    }

    // Local validation
    const error = validateExtension(value);
    setLocalErrors(prev => {
      const next = { ...prev };
      if (error) {
        next.extension = error;
      } else {
        delete next.extension;
      }
      return next;
    });
  }, [onChange, validateExtension]);

  const handlePatternChange = useCallback((value: string) => {
    setFilenamePattern(value);
    if (onChange) {
      onChange("filename_pattern", value);
    }
    // Clear pattern error
    setLocalErrors(prev => {
      const next = { ...prev };
      delete next.filename_pattern;
      return next;
    });
  }, [onChange]);

  const handleBrowse = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: "Select destination folder",
      });

      if (selectedPath) {
        handlePathChange(selectedPath as string);
      }
    } catch (error) {
      console.error("Directory picker error:", error);
      toast.error(`Failed to pick directory: ${error}`);
    }
  };

  const handleSave = async () => {
    if (!onSave || !persistOnChange) return;

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
    setLocalErrors({});
  };

  // Merge external and local errors
  const allErrors = { ...errors, ...localErrors };

  return (
    <div className="w-full space-y-4">
      <SettingsGroup
        title="FILE SYSTEM CONFIGURATION"
        description="Configure where and how transcriptions are saved"
      >
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
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => handlePathChange(e.target.value)}
              placeholder="~/UltraWhisper/Transcriptions"
              className={`flex-1 ${allErrors.path ? 'border-red-500' : ''}`}
              disabled={isSaving}
              aria-invalid={!!allErrors.path}
              aria-describedby={allErrors.path ? "path-error" : undefined}
            />
            <Button
              variant="secondary"
              onClick={handleBrowse}
              disabled={isSaving}
            >
              Browse…
            </Button>
          </div>
          {allErrors.path && (
            <span id="path-error" className="text-red-500 text-sm mt-1 block">
              {allErrors.path}
            </span>
          )}
          <div className="text-xs text-gray-500 mt-2">
            Use ~ for home directory. Directory will be created if it doesn't exist.
          </div>
        </SettingContainer>

        <SettingContainer
          title="File Extension"
          description="Extension for saved transcription files"
          layout="horizontal"
          descriptionMode="inline"
          grouped
        >
          <div className="w-32">
            <Input
              type="text"
              value={extension}
              onChange={handleExtensionChange}
              placeholder="md"
              className={allErrors.extension ? 'border-red-500' : ''}
              disabled={isSaving}
              maxLength={10}
              aria-invalid={!!allErrors.extension}
              aria-describedby={allErrors.extension ? "extension-error" : undefined}
            />
            {allErrors.extension && (
              <span id="extension-error" className="text-red-500 text-xs mt-1 block">
                {allErrors.extension}
              </span>
            )}
          </div>
        </SettingContainer>

        <SettingContainer
          title="Filename Pattern"
          description="Pattern for generated filenames. Available tokens: {timestamp}, {date}, {time}, {model_name}, {workflow_name}"
          layout="stacked"
          descriptionMode="inline"
          grouped
        >
          <Input
            type="text"
            value={filenamePattern}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => handlePatternChange(e.target.value)}
            placeholder="transcription_{timestamp}"
            className={allErrors.filename_pattern ? 'border-red-500' : ''}
            disabled={isSaving}
            aria-invalid={!!allErrors.filename_pattern}
            aria-describedby={allErrors.filename_pattern ? "pattern-error" : undefined}
          />
          {allErrors.filename_pattern && (
            <span id="pattern-error" className="text-red-500 text-sm mt-1 block">
              {allErrors.filename_pattern}
            </span>
          )}
          <div className="text-xs text-gray-500 mt-2">
            Example: transcription_{"{timestamp}"} → transcription_20240131_143022.md
          </div>
        </SettingContainer>

        {/* Show save/revert buttons only for existing destinations with changes */}
        {persistOnChange && isDirty && (
          <div className="flex gap-2 justify-end mt-4 pt-4 border-t">
            <Button
              variant="secondary"
              onClick={handleRevert}
              disabled={isSaving}
            >
              Revert
            </Button>
            <Button
              variant="primary"
              onClick={handleSave}
              disabled={isSaving || !path.trim() || !extension.trim() || !filenamePattern.trim()}
            >
              {isSaving ? "Saving..." : "Save Changes"}
            </Button>
          </div>
        )}
      </SettingsGroup>
    </div>
  );
}