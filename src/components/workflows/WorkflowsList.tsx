import React from "react";
import { Zap, ZapOff } from "lucide-react";
import { useWorkflows } from "../../hooks/useWorkflows";
import { StoredWorkflow } from "../../lib/types";
import { PrimaryButton } from "../shared";

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
        <div className="p-4 border-b uw-border-default">
          <h2 className="text-lg font-semibold uw-text-primary">Workflows</h2>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="uw-text-secondary">Loading workflows...</p>
        </div>
      </div>
    );
  }

  if (workflows.length === 0) {
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b uw-border-default">
          <h2 className="text-lg font-semibold uw-text-primary">Workflows</h2>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center p-4 gap-4">
          <p className="uw-text-secondary text-center">No workflows yet</p>
          <PrimaryButton onClick={handleNew}>
            + New Workflow
          </PrimaryButton>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b uw-border-default">
        <h2 className="text-lg font-semibold uw-text-primary">Workflows</h2>
        <PrimaryButton onClick={handleNew} fullWidth className="mt-3">
          + New Workflow
        </PrimaryButton>
      </div>
      <div className="flex-1 overflow-y-auto uw-scroll p-2">
        {workflows.map((workflow) => {
          const isActive = selectedId === workflow.id;
          const isEnabled = workflow.enabled;

          return (
            <div
              key={workflow.id}
              className={`
                p-3 rounded-lg cursor-pointer transition-all duration-150 mb-2
                border
                ${isActive
                  ? isEnabled
                    ? "uw-bg-primary-dim uw-border-primary uw-text-accent"
                    : "uw-bg-elevated border-gray-700 uw-text-secondary"
                  : isEnabled
                    ? "hover:uw-bg-card hover:border-gray-700 uw-text-primary border-transparent"
                    : "hover:bg-gray-900/30 uw-text-secondary border-transparent"
                }
              `}
              onClick={() => onSelect(workflow.id)}
            >
              <div className="flex items-start gap-2">
                {isEnabled ? (
                  <Zap className="w-4 h-4 uw-text-accent flex-shrink-0 mt-0.5" />
                ) : (
                  <ZapOff className="w-4 h-4 text-gray-600 flex-shrink-0 mt-0.5" />
                )}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    {!isEnabled && (
                      <span className="px-2 py-0.5 text-xs rounded-full uw-bg-card uw-text-dim border border-gray-700">
                        Disabled
                      </span>
                    )}
                  </div>
                  <p className={`text-sm font-medium truncate ${!isEnabled ? "opacity-60" : ""}`}>
                    {workflow.name}
                  </p>
                  {workflow.trigger?.type === "Hotkey" && (
                    <p className="text-xs uw-text-secondary mt-1 uw-mono">
                      {workflow.trigger.binding}
                    </p>
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
