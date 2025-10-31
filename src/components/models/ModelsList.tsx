import React from "react";

interface ModelsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const ModelsList: React.FC<ModelsListProps> = ({
  selectedId,
  onSelect,
}) => {
  // Simple placeholder - will show "Model Management" as single item
  const items = [{ id: "model-management", name: "Model Management" }];

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b uw-border-default">
        <h2 className="text-lg font-semibold uw-text-primary">Models</h2>
        <p className="text-xs uw-text-secondary mt-1">
          Manage Whisper models
        </p>
      </div>
      <div className="flex-1 overflow-y-auto uw-scroll p-2">
        {items.map((item) => (
          <div
            key={item.id}
            className={`p-3 rounded-lg cursor-pointer transition-colors mb-1 ${
              selectedId === item.id
                ? "uw-bg-primary-dim uw-text-accent"
                : "hover:uw-bg-card uw-text-primary"
            }`}
            onClick={() => onSelect(item.id)}
          >
            <p className="text-sm font-medium">{item.name}</p>
          </div>
        ))}
      </div>
    </div>
  );
};
