import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { TelegramSetup } from "./TelegramSetup";
import FileSystemSetup from "./FileSystemSetup";
import ActiveWindowSetup from "./ActiveWindowSetup";
import { ConfigCard, ConfigField, PrimaryButton } from "../shared";
import { Input } from "../ui/Input";

interface DestinationDetailProps {
  destinationId: string | null;
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

export const DestinationDetail: React.FC<DestinationDetailProps> = ({ destinationId }) => {
  const [loading, setLoading] = useState(false);
  const [dest, setDest] = useState<DestinationEntity | null>(null);

  // Name editing state
  const [name, setName] = useState("");
  const [originalName, setOriginalName] = useState("");
  const [isSavingName, setIsSavingName] = useState(false);

  const load = useCallback(async (id: string) => {
    setLoading(true);
    try {
      const result = await invoke<DestinationEntity | null>("get_destination", { id });
      setDest(result);
    } catch (e) {
      console.error("Failed to load destination:", e);
      toast.error("Failed to load destination");
      setDest(null);
    } finally {
      setLoading(false);
    }
  }, []);

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
      setOriginalName(dest.name);
    }
  }, [dest?.id, dest?.name]);

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

  // Name save handlers
  const handleSaveName = async () => {
    setIsSavingName(true);
    try {
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
    setName(originalName);
  };

  // Type-specific save handlers
  const handleSaveTelegram = async (newChatId: string) => {
    try {
      const updated: DestinationEntity = {
        ...dest,
        config: { ...dest.config, chat_id: newChatId } as DestinationConfig,
      };
      await invoke("update_destination", { destination: updated });
      toast.success("Telegram destination updated");
      setDest(updated);
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to update destination: ${msg}`);
    }
  };

  const handleSaveFileSystem = async (newConfig: typeof dest.config) => {
    try {
      const updated: DestinationEntity = {
        ...dest,
        config: newConfig,
      };
      await invoke("update_destination", { destination: updated });
      toast.success("File System destination updated");
      setDest(updated);
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to update destination: ${msg}`);
    }
  };

  const handleSaveActiveWindow = async (newConfig: typeof dest.config) => {
    try {
      const updated: DestinationEntity = {
        ...dest,
        config: newConfig,
      };
      await invoke("update_destination", { destination: updated });
      toast.success("Active Window destination updated");
      setDest(updated);
      window.dispatchEvent(new CustomEvent("destinations-changed"));
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to update destination: ${msg}`);
    }
  };

  // Single return with General section + type-specific editor
  return (
    <div className="flex-1 overflow-y-auto uw-scroll">
      <div className="p-6">
        {/* General Section - always rendered */}
        <ConfigCard title="General">
          <ConfigField
            label="Name"
            hint="Label shown in lists and pickers"
          >
            <Input
              value={name}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
              placeholder="My Destination"
              disabled={isSavingName}
              className="w-full"
            />
            {isNameDirty && (
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
            credentialId={dest.config.credential_id || "telegram_default"}
            initialChatId={dest.config.chat_id || ""}
            onSave={handleSaveTelegram}
          />
        )}

        {dest.config.type === "file_system" && (
          <FileSystemSetup config={dest.config} onSave={handleSaveFileSystem} />
        )}

        {dest.config.type === "active_window" && (
          <ActiveWindowSetup config={dest.config} onSave={handleSaveActiveWindow} />
        )}
      </div>
    </div>
  );
};
