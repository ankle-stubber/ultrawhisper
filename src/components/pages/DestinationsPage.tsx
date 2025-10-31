import { useState, useEffect } from "react";
import { DestinationsList } from "../destinations/DestinationsList";
import { DestinationDetail } from "../destinations/DestinationDetail";

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
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h1 className="text-2xl font-semibold text-gray-100">Destinations</h1>
        <p className="text-sm text-gray-400 mt-1">Configure output destinations for transcriptions</p>
      </div>

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel - Destinations List */}
        <div className="w-80 border-r border-gray-800 overflow-y-auto bg-gray-900/50">
          <DestinationsList
            selectedId={selectedDestinationId}
            onSelect={setSelectedDestinationId}
          />
        </div>

        {/* Right Panel - Destination Detail */}
        <div className="flex-1 overflow-y-auto">
          <DestinationDetail destinationId={selectedDestinationId} />
        </div>
      </div>
    </div>
  );
}