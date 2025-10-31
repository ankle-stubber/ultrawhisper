import React from "react";
import { WorkflowEditor } from "./WorkflowEditor";

interface WorkflowDetailProps {
  workflowId: string | null;
}

export const WorkflowDetail: React.FC<WorkflowDetailProps> = ({
  workflowId,
}) => {
  if (!workflowId) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Select a workflow to view details</p>
      </div>
    );
  }

  return (
    <div className="flex-1 min-w-0 w-full">
      <WorkflowEditor workflowId={workflowId} />
    </div>
  );
};
