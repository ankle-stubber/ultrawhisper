import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { FileText, Send, Monitor } from "lucide-react";
import { PrimaryButton } from "../shared";

interface DestinationsListProps {
  selectedId: string | null;
  onSelect: (id: string, creationType?: string) => void;
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
  const [isCreating, setIsCreating] = useState(false);
  const [showCreateMenu, setShowCreateMenu] = useState(false);
  const [ephemeralItem, setEphemeralItem] = useState<DestinationEntity | null>(null);
  const createMenuRef = useRef<HTMLDivElement>(null);
  const firstMenuItemRef = useRef<HTMLButtonElement>(null);

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

  // Listen for destination changes from other views
  useEffect(() => {
    const handleDestinationsChanged = () => {
      // Clear ephemeral item when destinations are refreshed (after creation)
      setEphemeralItem(null);
      load();
    };

    window.addEventListener("destinations-changed", handleDestinationsChanged);
    return () => window.removeEventListener("destinations-changed", handleDestinationsChanged);
  }, [load]);

  // Sorted view used for rendering and auto-select logic, includes ephemeral item
  const sorted = useMemo(() => {
    // Combine real items with ephemeral item
    const allItems = ephemeralItem
      ? [...items, ephemeralItem]
      : items;

    // Sort by type then name for a stable list
    return [...allItems].sort((a, b) => {
      // Ephemeral items come first
      if (a === ephemeralItem) return -1;
      if (b === ephemeralItem) return 1;

      const at = typeLabel(a.config);
      const bt = typeLabel(b.config);
      return at === bt ? a.name.localeCompare(b.name) : at.localeCompare(bt);
    });
  }, [items, ephemeralItem]);

  // Auto-select first item if nothing is selected or selected item doesn't exist
  useEffect(() => {
    if (!loading && sorted.length > 0) {
      // If no selection OR selected item not in list, select first from sorted list
      if (!selectedId || !sorted.find(item => item.id === selectedId)) {
        onSelect(sorted[0].id);
      }
    }
  }, [loading, sorted, selectedId, onSelect]);

  const handleCreateDestination = useCallback((type: "telegram" | "file_system" | "active_window") => {
    setShowCreateMenu(false);

    // Generate ID locally
    const id = crypto.randomUUID();

    const nameDefaults = {
      active_window: "Active Window",
      file_system: "File System",
      telegram: "Telegram",
    };

    // Create ephemeral item to show in list (unsaved indicator)
    const ephemeral: DestinationEntity = {
      id,
      name: `New ${nameDefaults[type]} (unsaved)`,
      config: {
        type: type as any,
        // Minimal config just for display
      } as DestinationConfig,
    };

    setEphemeralItem(ephemeral);

    // Navigate to detail panel with creation intent
    // The creationType parameter tells DestinationDetail to enter creation mode
    onSelect(id, type);
  }, [onSelect]);

  // Click outside and Escape handlers for create menu
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (createMenuRef.current && !createMenuRef.current.contains(event.target as Node)) {
        setShowCreateMenu(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setShowCreateMenu(false);
      }
    };

    if (showCreateMenu) {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleEscape);
      return () => {
        document.removeEventListener("mousedown", handleClickOutside);
        document.removeEventListener("keydown", handleEscape);
      };
    }
  }, [showCreateMenu]);

  // Focus first menu item when menu opens
  useEffect(() => {
    if (showCreateMenu && firstMenuItemRef.current) {
      firstMenuItemRef.current.focus();
    }
  }, [showCreateMenu]);



  // Create menu component
  const CreateMenu: React.FC<{ onSelect: (type: "telegram" | "file_system" | "active_window") => void }> = ({ onSelect }) => (
    <div className="absolute right-0 top-full mt-1 uw-bg-elevated border uw-border-default rounded shadow-lg z-50 min-w-[160px]">
      <button
        ref={firstMenuItemRef}
        onClick={() => onSelect("telegram")}
        className="w-full px-3 py-2 text-sm text-left uw-text-primary hover:uw-bg-primary-dim transition-colors focus:uw-bg-primary-dim focus:outline-none"
      >
        Telegram
      </button>
      <button
        onClick={() => onSelect("file_system")}
        className="w-full px-3 py-2 text-sm text-left uw-text-primary hover:uw-bg-primary-dim transition-colors border-t uw-border-subtle focus:uw-bg-primary-dim focus:outline-none"
      >
        File System
      </button>
      <button
        onClick={() => onSelect("active_window")}
        className="w-full px-3 py-2 text-sm text-left uw-text-primary hover:uw-bg-primary-dim transition-colors border-t uw-border-subtle focus:uw-bg-primary-dim focus:outline-none"
      >
        Active Window
      </button>
    </div>
  );

  return (
    <div className="flex flex-col h-full">
      {loading ? (
        <>
          <div className="p-4 border-b uw-border-default">
            <h2 className="text-lg font-semibold uw-text-primary">Destinations</h2>
          </div>
          <div className="flex-1 flex items-center justify-center">
            <p className="uw-text-secondary">Loading destinations...</p>
          </div>
        </>
      ) : sorted.length === 0 ? (
        <>
          <div className="p-4 border-b uw-border-default">
            <h2 className="text-lg font-semibold uw-text-primary">Destinations</h2>
          </div>
          <div className="flex-1 flex flex-col items-center justify-center p-4 gap-4">
            <p className="uw-text-secondary text-center">No destinations yet</p>
            <div className="relative" ref={createMenuRef}>
              <PrimaryButton
                onClick={() => setShowCreateMenu(!showCreateMenu)}
                disabled={isCreating}
              >
                {isCreating ? "Creating..." : "+ New Destination"}
              </PrimaryButton>
              {showCreateMenu && <CreateMenu onSelect={handleCreateDestination} />}
            </div>
          </div>
        </>
      ) : (
        <>
          <div className="p-4 border-b uw-border-default">
            <h2 className="text-lg font-semibold uw-text-primary">Destinations</h2>
            <div className="relative mt-3" ref={createMenuRef}>
              <PrimaryButton
                onClick={() => setShowCreateMenu(!showCreateMenu)}
                disabled={isCreating}
                fullWidth
              >
                + New Destination
              </PrimaryButton>
              {showCreateMenu && <CreateMenu onSelect={handleCreateDestination} />}
            </div>
          </div>
          <div className="flex-1 overflow-y-auto uw-scroll p-2">
        {!loading &&
          sorted.map((dest) => {
            const isActive = selectedId === dest.id;
            const isEphemeral = dest === ephemeralItem;
            const type = dest.config.type;

            // Type icons and colors
            const typeConfig = {
              telegram: { icon: Send, color: "text-blue-500", bgColor: "bg-blue-500/10", borderColor: "border-blue-500/20" },
              file_system: { icon: FileText, color: "text-amber-500", bgColor: "bg-amber-500/10", borderColor: "border-amber-500/20" },
              active_window: { icon: Monitor, color: "text-green-500", bgColor: "bg-green-500/10", borderColor: "border-green-500/20" },
            };

            const config = typeConfig[type] || typeConfig.active_window;
            const Icon = config.icon;

            return (
              <div
                key={dest.id}
                className={`
                  p-3 rounded-lg cursor-pointer transition-all duration-150 mb-2
                  border
                  ${isActive
                    ? "uw-bg-primary-dim uw-border-primary uw-text-accent"
                    : "hover:uw-bg-card hover:border-gray-700 uw-text-primary border-transparent"
                  }
                  ${isEphemeral ? "opacity-70 border-dashed" : ""}
                `}
                onClick={() => onSelect(dest.id)}
              >
                <div className="flex items-center gap-3">
                  <Icon className={`w-4 h-4 ${isActive ? "uw-text-accent" : config.color} flex-shrink-0`} />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">
                      {dest.name}
                      {isEphemeral && (
                        <span className="ml-2 text-xs text-yellow-500">(unsaved)</span>
                      )}
                    </p>
                    <div className="flex items-center gap-2 mt-1">
                      <span className={`text-xs px-2 py-0.5 rounded-full ${config.bgColor} ${config.borderColor} border`}>
                        {typeLabel(dest.config)}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
          </div>
        </>
      )}
    </div>
  );
};
