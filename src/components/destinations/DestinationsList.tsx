import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

interface DestinationsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
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

function typeLabel(config: DestinationConfig): string {
  switch (config.type) {
    case "active_window":
      return "Active Window";
    case "file_system":
      return "File System";
    case "telegram":
      return "Telegram";
    default:
      return "Unknown";
  }
}

export const DestinationsList: React.FC<DestinationsListProps> = ({
  selectedId,
  onSelect,
}) => {
  const [loading, setLoading] = useState(false);
  const [items, setItems] = useState<DestinationEntity[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<DestinationEntity[]>("list_destinations");
      setItems(list);
    } catch (e) {
      console.error("Failed to load destinations:", e);
      toast.error("Failed to load destinations");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleAddTelegram = useCallback(async () => {
    try {
      // If telegram_default exists, just select it
      const existing = items.find(
        (d) => d.id === "telegram_default" || (d.config as any).type === "telegram"
      );
      if (existing) {
        onSelect(existing.id);
        return;
      }

      const newDest: DestinationEntity = {
        id: "telegram_default",
        name: "Telegram",
        config: {
          type: "telegram",
          credential_id: "telegram_default",
          chat_id: "",
          include_audio: false,
        },
        template: "[{timestamp}] {workflow_name}\n\n{transcription_text}",
      };

      await invoke("create_destination", { destination: newDest });
      toast.success("Telegram destination created");
      await load();
      onSelect("telegram_default");
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.toString?.() || "Unknown error";
      toast.error(`Failed to create Telegram destination: ${msg}`);
    }
  }, [items, load, onSelect]);

  const sorted = useMemo(() => {
    // Sort by type then name for a stable list
    return [...items].sort((a, b) => {
      const at = typeLabel(a.config);
      const bt = typeLabel(b.config);
      return at === bt ? a.name.localeCompare(b.name) : at.localeCompare(bt);
    });
  }, [items]);

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-mid-gray/20 flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Destinations</h2>
          <p className="text-xs text-mid-gray mt-1">
            Configure where transcriptions are sent
          </p>
        </div>
        <button
          onClick={handleAddTelegram}
          className="px-2 py-1 text-xs rounded-md bg-blue-600 text-white hover:bg-blue-700"
          title="Add Telegram destination"
        >
          + Telegram
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {loading && (
          <div className="text-xs text-mid-gray p-2">Loading…</div>
        )}
        {!loading && sorted.length === 0 && (
          <div className="text-xs text-mid-gray p-2">No destinations found</div>
        )}
        {!loading &&
          sorted.map((dest) => (
            <div
              key={dest.id}
              className={`p-3 rounded-lg cursor-pointer transition-colors mb-1 ${
                selectedId === dest.id
                  ? "bg-logo-primary/80"
                  : "hover:bg-mid-gray/20"
              }`}
              onClick={() => onSelect(dest.id)}
            >
              <p className="text-sm font-medium">{dest.name}</p>
              <p className="text-xs text-mid-gray mt-0.5">{typeLabel(dest.config)}</p>
            </div>
          ))}
      </div>
    </div>
  );
};
