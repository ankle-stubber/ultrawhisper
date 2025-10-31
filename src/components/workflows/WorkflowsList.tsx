import React from "react";
import { Zap, ZapOff, Keyboard } from "lucide-react";
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
        {workflows.map((workflow) => {
          const isActive = selectedId === workflow.id;
          const isEnabled = workflow.enabled;

          return (
            <div
              key={workflow.id}
              className={`
                p-3 rounded-lg cursor-pointer transition-all duration-150 mb-2
                border border-transparent
                ${isActive
                  ? isEnabled
                    ? "bg-green-500/10 border-green-500/30 text-green-50"
                    : "bg-gray-800 border-gray-700 text-gray-300"
                  : isEnabled
                    ? "hover:bg-gray-900/50 hover:border-gray-700 text-gray-200"
                    : "hover:bg-gray-900/30 text-gray-500"
                }
              `}
              onClick={() => onSelect(workflow.id)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 min-w-0">
                  {isEnabled ? (
                    <Zap className="w-4 h-4 text-green-500 flex-shrink-0" />
                  ) : (
                    <ZapOff className="w-4 h-4 text-gray-600 flex-shrink-0" />
                  )}
                  <p className={`text-sm font-medium truncate ${!isEnabled ? "opacity-60" : ""}`}>
                    {workflow.name}
                  </p>
                </div>
                <div className="flex items-center gap-2 flex-shrink-0">
                  {workflow.trigger?.type === "Hotkey" && (
                    <div className="flex items-center gap-1">
                      <Keyboard className="w-3 h-3 text-gray-500" />
                      <span className="text-xs font-mono text-gray-400">
                        {workflow.trigger.binding}
                      </span>
                    </div>
                  )}
                  {!isEnabled && (
                    <span className="text-xs text-gray-500 bg-gray-800 px-2 py-0.5 rounded">
                      Off
                    </span>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
