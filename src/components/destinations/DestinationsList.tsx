import React from "react";

interface DestinationsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const DestinationsList: React.FC<DestinationsListProps> = ({
  selectedId,
  onSelect,
}) => {
  // Placeholder - will be replaced with actual destination loading in Phase 3
  const destinations = [
    { id: "telegram-default", name: "Telegram", type: "Telegram" },
  ];

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-mid-gray/20">
        <h2 className="text-lg font-semibold">Destinations</h2>
        <p className="text-xs text-mid-gray mt-1">
          Configure where transcriptions are sent
        </p>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {destinations.map((dest) => (
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
            <p className="text-xs text-mid-gray mt-0.5">{dest.type}</p>
          </div>
        ))}
      </div>
    </div>
  );
};
