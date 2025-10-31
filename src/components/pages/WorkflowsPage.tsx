import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { WorkflowsList } from "../workflows/WorkflowsList";
import { WorkflowDetail } from "../workflows/WorkflowDetail";
import { PageHeader } from "../shared";

export function WorkflowsPage() {
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);

  // Auto-select first workflow
  useEffect(() => {
    // Let WorkflowsList handle the initial selection
    setSelectedWorkflowId("batch-processing");
  }, []);

  // Listen for workflow changes
  useEffect(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const unlisten = await listen("workflows-changed", () => {
        console.log("Workflows changed event received");
      });
      cleanups.push(unlisten);
    })();
    return () => {
      cleanups.forEach((fn) => fn());
    };
  }, []);

  return (
    <div className="flex-1 flex flex-col h-full uw-bg-surface">
      {/* Header */}
      <PageHeader
        title="Workflows"
        subtitle="Configure voice-to-action automation"
      />

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel - Workflows List */}
        <div className="w-80 border-r uw-border-default overflow-y-auto uw-scroll uw-bg-elevated">
          <WorkflowsList
            selectedId={selectedWorkflowId}
            onSelect={setSelectedWorkflowId}
          />
        </div>

        {/* Right Panel - Workflow Detail */}
        <div className="flex-1 overflow-y-auto uw-scroll">
          <WorkflowDetail workflowId={selectedWorkflowId} />
        </div>
      </div>
    </div>
  );
}
