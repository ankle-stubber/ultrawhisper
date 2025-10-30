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
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Select a workflow to view details</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto min-w-0 min-h-0">
      <WorkflowEditor workflowId={workflowId} />
    </div>
  );
};
