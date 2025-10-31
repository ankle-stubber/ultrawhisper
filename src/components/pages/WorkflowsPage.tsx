import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { WorkflowsList } from "../workflows/WorkflowsList";
import { WorkflowDetail } from "../workflows/WorkflowDetail";

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
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h1 className="text-2xl font-semibold text-gray-100">Workflows</h1>
        <p className="text-sm text-gray-400 mt-1">Configure voice-to-action automation</p>
      </div>

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel - Workflows List */}
        <div className="w-80 border-r border-gray-800 overflow-y-auto bg-gray-900/50">
          <WorkflowsList
            selectedId={selectedWorkflowId}
            onSelect={setSelectedWorkflowId}
          />
        </div>

        {/* Right Panel - Workflow Detail */}
        <div className="flex-1 overflow-y-auto">
          <WorkflowDetail workflowId={selectedWorkflowId} />
        </div>
      </div>
    </div>
  );
}
