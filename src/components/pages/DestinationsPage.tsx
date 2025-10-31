import { useState, useEffect } from "react";
import { DestinationsList } from "../destinations/DestinationsList";
import { DestinationDetail } from "../destinations/DestinationDetail";
import { PageHeader } from "../shared";

export function DestinationsPage() {
  const [selectedDestinationId, setSelectedDestinationId] = useState<string | null>(null);

  // Listen for destination changes (DOM events from existing components)
  useEffect(() => {
    const handleDestinationsChanged = () => {
      // DestinationsList will automatically refresh
      console.log("Destinations changed event received");
    };

    window.addEventListener("destinations-changed", handleDestinationsChanged);

    return () => {
      window.removeEventListener("destinations-changed", handleDestinationsChanged);
    };
  }, []);

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
            onSelect={setSelectedDestinationId}
          />
        </div>

        {/* Right Panel - Destination Detail */}
        <div className="flex-1 overflow-y-auto uw-scroll">
          <DestinationDetail destinationId={selectedDestinationId} />
        </div>
      </div>
    </div>
  );
}