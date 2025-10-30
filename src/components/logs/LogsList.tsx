import React from "react";

interface LogsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const LogsList: React.FC<LogsListProps> = ({
  selectedId,
  onSelect,
}) => {
  // Placeholder - will be replaced with actual log viewer in Phase 4
  const items = [{ id: "application-logs", name: "Application Logs" }];

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-mid-gray/20">
        <h2 className="text-lg font-semibold">Logs</h2>
        <p className="text-xs text-mid-gray mt-1">
          View application logs
        </p>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {items.map((item) => (
          <div
            key={item.id}
            className={`p-3 rounded-lg cursor-pointer transition-colors mb-1 ${
              selectedId === item.id
                ? "bg-logo-primary/80"
                : "hover:bg-mid-gray/20"
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
