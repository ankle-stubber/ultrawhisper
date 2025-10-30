import React from "react";

interface WorkflowsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const WorkflowsList: React.FC<WorkflowsListProps> = ({
  selectedId,
  onSelect,
}) => {
  // For MVP, we only have "Batch Processing (legacy)" as a single workflow item
  const workflows = [
    { id: "batch-processing", name: "Batch Processing (legacy)" },
  ];

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-mid-gray/20">
        <h2 className="text-lg font-semibold">Workflows</h2>
        <p className="text-xs text-mid-gray mt-1">
          Full workflow editor coming soon
        </p>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {workflows.map((workflow) => (
          <div
            key={workflow.id}
            className={`p-3 rounded-lg cursor-pointer transition-colors mb-1 ${
              selectedId === workflow.id
                ? "bg-logo-primary/80"
                : "hover:bg-mid-gray/20"
            }`}
            onClick={() => onSelect(workflow.id)}
          >
            <p className="text-sm font-medium">{workflow.name}</p>
          </div>
        ))}
      </div>
    </div>
  );
};
