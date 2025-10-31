import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { ModelsList } from "../models/ModelsList";
import { ModelDetail } from "../models/ModelDetail";
import ModelSelector from "../model-selector";
import { PageHeader, PrimaryButton } from "../shared";

export function ModelsPage() {
  const [selectedModelId, setSelectedModelId] = useState<string | null>("model-management");
  const [showSimpleView, setShowSimpleView] = useState(true);

  // Listen for model state changes
  useEffect(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const unlisten = await listen("model-state-changed", (event) => {
        console.log("Model state changed:", event.payload);
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
        title="Models"
        subtitle="Manage transcription models"
        actions={
          <PrimaryButton
            onClick={() => setShowSimpleView(!showSimpleView)}
            variant="secondary"
            size="sm"
          >
            {showSimpleView ? "Advanced View" : "Simple View"}
          </PrimaryButton>
        }
      />

      {/* Content */}
      {showSimpleView ? (
        // Simple view - just the ModelSelector
        <div className="flex-1 overflow-y-auto uw-scroll p-6">
          <div className="max-w-4xl mx-auto">
            <ModelSelector />
          </div>
        </div>
      ) : (
        // Advanced view - Two Panel Layout
        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Models List */}
          <div className="w-80 border-r uw-border-default overflow-y-auto uw-scroll uw-bg-elevated">
            <ModelsList
              selectedId={selectedModelId}
              onSelect={setSelectedModelId}
            />
          </div>

          {/* Right Panel - Model Detail */}
          <div className="flex-1 overflow-y-auto uw-scroll">
            <ModelDetail modelId={selectedModelId} />
          </div>
        </div>
      )}
    </div>
  );
}
