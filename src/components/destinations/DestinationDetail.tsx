import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { homeDir, join } from "@tauri-apps/api/path";
import { TelegramSetup } from "./TelegramSetup";
import FileSystemSetup from "./FileSystemSetup";
import ActiveWindowSetup from "./ActiveWindowSetup";
import { ConfigCard, ConfigField, PrimaryButton } from "../shared";
import { Input } from "../ui/Input";
import {
  validateDestination,
  mapBackendErrors,
  isFormValid,
  getDefaultTemplate,
  typeLabel,
} from "../../lib/destinations/validation";

interface DestinationDetailProps {
  destinationId: string | null;
  creationType?: string; // New prop for creation mode
}

type DestinationConfig =
  | { type: "active_window"; paste_method: "ctrl_v" | "direct"; preserve_clipboard: boolean }
  | { type: "file_system"; path: string; extension: string; filename_pattern: string }
  | { type: "telegram"; credential_id: string; chat_id: string; include_audio: boolean };

interface DestinationEntity {
  id: string;
  name: string;
  config: DestinationConfig;
  template?: string | null;
}

export const DestinationDetail: React.FC<DestinationDetailProps> = ({
  destinationId,
  creationType
}) => {
  const [loading, setLoading] = useState(false);
  const [dest, setDest] = useState<DestinationEntity | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [isDirty, setIsDirty] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Name editing state
  const [name, setName] = useState("");
  const [originalName, setOriginalName] = useState("");
  const [isSavingName, setIsSavingName] = useState(false);

  // Generate OS-aware defaults for new destinations
  const createDefaultDestination = useCallback(async (
    id: string,
    type: string
  ): Promise<DestinationEntity> => {
    let homePath = "~";
    try {
      homePath = await homeDir();
    } catch {
      // Fallback handled below
    }

    let defaultPath = `${homePath}/UltraWhisper/Transcriptions`;
    try {
      defaultPath = await join(homePath, "UltraWhisper", "Transcriptions");
    } catch {
      // join may fail if homePath is a tilde — fall back to simple concat
      defaultPath = `${homePath}/UltraWhisper/Transcriptions`;
    }

    const defaults: Record<string, DestinationConfig> = {
      file_system: {
        type: "file_system",
        path: defaultPath,
        extension: "md",
        filename_pattern: "transcription_{timestamp}",
      },
      telegram: {
        type: "telegram",
        credential_id: "", // Require explicit selection
        chat_id: "",
        include_audio: false,
      },
      active_window: {
        type: "active_window",
        // Backend interprets ctrl_v as Cmd+V on macOS, so this remains portable
        paste_method: "ctrl_v",
        preserve_clipboard: false,
      },
    };

    return {
      id,
      name: `New ${typeLabel(type)}`,
      config: defaults[type] || defaults.active_window,
      template: getDefaultTemplate(type),
    };
  }, []);

  const load = useCallback(async (id: string) => {
    setLoading(true);
    try {
      const result = await invoke<DestinationEntity | null>("get_destination", { id });
      if (result) {
        setDest(result);
        setIsNew(false);
      } else if (creationType) {
        // Destination doesn't exist and we have creation intent
        const newDest = await createDefaultDestination(id, creationType);
        setDest(newDest);
        setIsNew(true);
        setIsDirty(true); // Mark as dirty to prevent accidental navigation
      } else {
        setDest(null);
      }
    } catch (e) {
      // If get_destination fails, check for creation intent
      if (creationType) {
        const newDest = await createDefaultDestination(id, creationType);
        setDest(newDest);
        setIsNew(true);
        setIsDirty(true);
      } else {
        console.error("Failed to load destination:", e);
        toast.error("Failed to load destination");
        setDest(null);
      }
    } finally {
      setLoading(false);
    }
  }, [creationType, createDefaultDestination]);

  useEffect(() => {
    if (destinationId) {
      load(destinationId);
    } else {
      setDest(null);
    }
  }, [destinationId, load]);

  // Update name state when destination changes
  useEffect(() => {
    if (dest) {
      setName(dest.name);
      setOriginalName(isNew ? "" : dest.name);
    }
  }, [dest?.id, dest?.name, isNew]);

  // Validate on changes
  useEffect(() => {
    if (dest && isDirty) {
      const validationErrors = validateDestination({
        ...dest,
        name: name.trim(),
      });
      setErrors(validationErrors);
    }
  }, [dest, name, isDirty]);

  // Field change handler
  const handleFieldChange = useCallback((field: string, value: any) => {
    setDest(prev => {
      if (!prev) return null;

      const updated = {
        ...prev,
        config: { ...prev.config, [field]: value },
      };

      // Clear field-specific error
      setErrors(prevErrors => {
        const newErrors = { ...prevErrors };
        delete newErrors[field];
        return newErrors;
      });

      setIsDirty(true);
      return updated;
    });
  }, []);

  // Create new destination
  const handleCreate = async () => {
    if (!dest) return;

    const finalDest = { ...dest, name: name.trim() };

    // Validate locally first
    const validationErrors = validateDestination(finalDest);
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors);
      toast.error("Please fix the highlighted errors");
      return;
    }

    setIsCreating(true);
    try {
      await invoke("create_destination", { destination: finalDest });

      toast.success("Destination created");
      setIsNew(false);
      setIsDirty(false);
      setOriginalName(name.trim());

      // Notify list to refresh
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      // Map backend errors to fields
      const errorMapping = mapBackendErrors(e);
      setErrors(errorMapping);

      const generalError = errorMapping._general;
      if (generalError) {
        toast.error(generalError);
      } else {
        toast.error("Please fix the highlighted errors");
      }
    } finally {
      setIsCreating(false);
    }
  };

  // Cancel creation
  const handleCancel = async () => {
    if (isDirty) {
      const confirmed = window.confirm("You have unsaved changes. Discard them?");
      if (!confirmed) return;
    }

    // Clear ephemeral item and navigate back
    // Since we don't have routing here, we'll just clear the selection
    // The parent component should handle navigation
    window.dispatchEvent(new CustomEvent("destination-creation-cancelled"));
  };

  // Name save handlers (for existing destinations)
  const handleSaveName = async () => {
    if (isNew) return; // Use Create button for new destinations

    setIsSavingName(true);
    try {
      if (!dest) return;

      const updated: DestinationEntity = {
        ...dest,
        name: name.trim(),
      };
      await invoke("update_destination", { destination: updated });
      toast.success("Destination renamed");
      setDest(updated);
      setOriginalName(name.trim());

      // Notify other views
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to rename: ${msg}`);
    } finally {
      setIsSavingName(false);
    }
  };

  const handleRevertName = () => {
    setName(originalName || dest?.name || "");
  };

  // Type-specific save handlers (for existing destinations)
  const handleSaveConfig = async (newConfig: DestinationConfig) => {
    if (!dest || isNew) return;

    try {
      const updated: DestinationEntity = {
        ...dest,
        config: newConfig,
      };
      await invoke("update_destination", { destination: updated });
      toast.success("Destination updated");
      setDest(updated);
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to update destination: ${msg}`);
    }
  };

  // Early returns for loading states
  if (!destinationId) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Select a destination to configure</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Loading…</p>
      </div>
    );
  }

  if (!dest) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Destination not found</p>
      </div>
    );
  }

  // Computed values
  const isNameDirty = name.trim() !== originalName;
  const canCreate = isNew && isFormValid({ ...dest, name: name.trim() }) && !isCreating;

  // Single return with creation indicator, general section, type-specific editor, and action buttons
  return (
    <div className="flex-1 overflow-y-auto uw-scroll">
      <div className="p-6">
        {/* Creation mode indicator */}
        {isNew && (
          <div className="bg-blue-500/10 text-blue-500 p-3 rounded mb-4 flex items-center gap-2">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
            <span>Creating new {dest.config.type.replace("_", " ")} destination</span>
          </div>
        )}

        {/* General Section - always rendered */}
        <ConfigCard title="General">
          <ConfigField
            label="Name"
            hint="Label shown in lists and pickers"
          >
            <Input
              value={name}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                setName(e.target.value);
                setIsDirty(true);
              }}
              placeholder="My Destination"
              disabled={isSavingName}
              className={`w-full ${errors.name ? 'border-red-500' : ''}`}
              aria-invalid={!!errors.name}
              aria-describedby={errors.name ? "name-error" : undefined}
            />
            {errors.name && (
              <span id="name-error" className="text-red-500 text-sm mt-1">
                {errors.name}
              </span>
            )}
            {!isNew && isNameDirty && (
              <div className="flex gap-2 justify-end mt-2">
                <PrimaryButton
                  variant="secondary"
                  onClick={handleRevertName}
                  disabled={isSavingName}
                  size="sm"
                >
                  Revert
                </PrimaryButton>
                <PrimaryButton
                  variant="primary"
                  onClick={handleSaveName}
                  disabled={!name.trim() || isSavingName}
                  size="sm"
                >
                  {isSavingName ? "Saving..." : "Save"}
                </PrimaryButton>
              </div>
            )}
          </ConfigField>
        </ConfigCard>

        {/* Type-specific editor */}
        {dest.config.type === "telegram" && (
          <TelegramSetup
            config={dest.config}
            onChange={isNew ? handleFieldChange : undefined}
            onSave={isNew ? undefined : handleSaveConfig}
            errors={errors}
            requireCredentialSelection={isNew}
          />
        )}

        {dest.config.type === "file_system" && (
          <FileSystemSetup
            config={dest.config}
            onChange={isNew ? handleFieldChange : undefined}
            onSave={isNew ? undefined : handleSaveConfig}
            errors={errors}
            persistOnChange={!isNew}
          />
        )}

        {dest.config.type === "active_window" && (
          <ActiveWindowSetup
            config={dest.config}
            onChange={isNew ? handleFieldChange : undefined}
            onSave={isNew ? undefined : handleSaveConfig}
            errors={errors}
          />
        )}

        {/* Action buttons for new destinations */}
        {isNew && (
          <div className="flex gap-2 mt-6">
            <PrimaryButton
              variant="primary"
              onClick={handleCreate}
              disabled={!canCreate}
              aria-busy={isCreating}
            >
              {isCreating ? "Creating..." : "Create Destination"}
            </PrimaryButton>
            <PrimaryButton
              variant="secondary"
              onClick={handleCancel}
              disabled={isCreating}
            >
              Cancel
            </PrimaryButton>
          </div>
        )}

        {/* Accessible error summary */}
        {Object.keys(errors).filter(k => k !== "_general").length > 0 && (
          <div role="alert" className="mt-4 p-3 bg-red-500/10 text-red-500 rounded">
            Please fix the errors above before creating the destination.
          </div>
        )}
      </div>
    </div>
  );
};
