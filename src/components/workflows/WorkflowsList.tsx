import React from "react";
import { useWorkflows } from "../../hooks/useWorkflows";
import { StoredWorkflow } from "../../lib/types";

interface WorkflowsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const WorkflowsList: React.FC<WorkflowsListProps> = ({
  selectedId,
  onSelect,
}) => {
  const { workflows, loading } = useWorkflows();

  const handleNew = () => {
    const newId = crypto.randomUUID();
    onSelect(newId);
  };

  if (loading) {
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b border-mid-gray/20">
          <h2 className="text-lg font-semibold">Workflows</h2>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-mid-gray">Loading workflows...</p>
        </div>
      </div>
    );
  }

  if (workflows.length === 0) {
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b border-mid-gray/20">
          <h2 className="text-lg font-semibold">Workflows</h2>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center p-4 gap-4">
          <p className="text-mid-gray text-center">No workflows yet</p>
          <button
            onClick={handleNew}
            className="px-4 py-2 bg-logo-primary hover:bg-logo-primary/80 rounded-lg transition-colors"
          >
            + New Workflow
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-mid-gray/20">
        <h2 className="text-lg font-semibold">Workflows</h2>
        <button
          onClick={handleNew}
          className="mt-2 w-full px-3 py-2 bg-logo-primary/20 hover:bg-logo-primary/30 rounded-lg transition-colors text-sm"
        >
          + New Workflow
        </button>
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
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">{workflow.name}</p>
              {!workflow.enabled && (
                <span className="text-xs text-mid-gray/60 bg-mid-gray/20 px-2 py-1 rounded">
                  Disabled
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
