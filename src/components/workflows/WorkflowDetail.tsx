import React from "react";
import BatchProcessingSettings from "../settings/BatchProcessingSettings";

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

  // For MVP, only batch-processing workflow exists
  if (workflowId === "batch-processing") {
    return (
      <div className="flex-1 overflow-y-auto">
        <div className="flex flex-col items-center p-4 gap-4">
          <BatchProcessingSettings />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-full text-mid-gray">
      <p>Workflow not found</p>
    </div>
  );
};
