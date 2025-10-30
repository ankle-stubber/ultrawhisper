import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { TelegramSetup } from "./TelegramSetup";

interface DestinationDetailProps {
  destinationId: string | null;
}

type DestinationConfig =
  | { type: "active_window"; paste_method: string; preserve_clipboard: boolean }
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

  if (!destinationId) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Select a destination to configure</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Loading…</p>
      </div>
    );
  }

  if (!dest) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Destination not found</p>
      </div>
    );
  }

  if (dest.config.type === "telegram") {
    const credId = dest.config.credential_id || "telegram_default";
    const chatId = dest.config.chat_id || "";

    const handleSave = async (newChatId: string) => {
      try {
        const updated: DestinationEntity = {
          ...dest,
          config: { ...dest.config, chat_id: newChatId } as DestinationConfig,
        };
        await invoke("update_destination", { destination: updated });
        toast.success("Telegram destination updated");
        setDest(updated);
      } catch (e: any) {
        const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
        toast.error(`Failed to update destination: ${msg}`);
      }
    };

    return (
      <div className="flex-1 overflow-y-auto">
        <div className="flex flex-col items-center p-4 gap-4">
          <TelegramSetup
            credentialId={credId}
            initialChatId={chatId}
            onSave={handleSave}
          />
        </div>
      </div>
    );
  }

  // Placeholder for other destination types
  return (
    <div className="flex items-center justify-center h-full text-mid-gray">
      <div className="p-4 text-center">
        <p className="text-sm font-medium">{dest.name}</p>
        <p className="text-xs text-mid-gray mt-1">Editing for this destination type coming soon</p>
      </div>
    </div>
  );
};
