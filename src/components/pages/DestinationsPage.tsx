import { useState, useEffect } from "react";
import { DestinationsList } from "../destinations/DestinationsList";
import { DestinationDetail } from "../destinations/DestinationDetail";
import { PageHeader } from "../shared";

const STORAGE_KEY = "ultrawhisper.destinations.selection";

export function DestinationsPage() {
  const [selectedDestinationId, setSelectedDestinationId] = useState<string | null>(null);
  const [creationType, setCreationType] = useState<string | undefined>(undefined);

  // Initialize from session storage on mount so state survives refreshes.
  useEffect(() => {
    try {
      const stored = sessionStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as { id?: string | null; type?: string | undefined };
        if (parsed.id) {
          setSelectedDestinationId(parsed.id);
        }
        if (parsed.type) {
          setCreationType(parsed.type);
        }
      }
    } catch (error) {
      console.warn("Failed to read destination selection from storage:", error);
    }
  }, []);

  // Handle selection with optional creation type
  const handleSelect = (id: string, type?: string) => {
    setSelectedDestinationId(id);
    setCreationType(type);

    try {
      sessionStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ id, type: type ?? undefined })
      );
    } catch (error) {
      console.warn("Failed to persist destination selection:", error);
    }
  };

  // Listen for destination changes (DOM events from existing components)
  useEffect(() => {
    const handleDestinationsChanged = () => {
      // DestinationsList will automatically refresh
      console.log("Destinations changed event received");
      // Clear creation type when a destination is successfully created
      setCreationType(undefined);
      // Update URL to remove newType param
      if (selectedDestinationId) {
        try {
          sessionStorage.setItem(
            STORAGE_KEY,
            JSON.stringify({ id: selectedDestinationId })
          );
        } catch (error) {
          console.warn("Failed to persist destination selection:", error);
        }
      }
    };

    const handleCreationCancelled = () => {
      // Clear selection when creation is cancelled
      setSelectedDestinationId(null);
      setCreationType(undefined);
      try {
        sessionStorage.removeItem(STORAGE_KEY);
      } catch (error) {
        console.warn("Failed to clear destination selection:", error);
      }
    };

    window.addEventListener("destinations-changed", handleDestinationsChanged);
    window.addEventListener("destination-creation-cancelled", handleCreationCancelled);

    return () => {
      window.removeEventListener("destinations-changed", handleDestinationsChanged);
      window.removeEventListener("destination-creation-cancelled", handleCreationCancelled);
    };
  }, [selectedDestinationId]);

  return (
    <div className="flex-1 flex flex-col h-full uw-bg-surface">
      {/* Header */}
      <PageHeader
        title="Destinations"
        subtitle="Configure output destinations for transcriptions"
      />

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel - Destinations List */}
        <div className="w-80 border-r uw-border-default overflow-y-auto uw-scroll uw-bg-elevated">
          <DestinationsList
            selectedId={selectedDestinationId}
            onSelect={handleSelect}
          />
        </div>

        {/* Right Panel - Destination Detail */}
        <div className="flex-1 overflow-y-auto uw-scroll">
          <DestinationDetail
            destinationId={selectedDestinationId}
            creationType={creationType}
          />
        </div>
      </div>
    </div>
  );
}
