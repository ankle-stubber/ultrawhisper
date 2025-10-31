import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { ModelsList } from "../models/ModelsList";
import { ModelDetail } from "../models/ModelDetail";
import ModelSelector from "../model-selector";

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
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold text-gray-100">Models</h1>
            <p className="text-sm text-gray-400 mt-1">Manage transcription models</p>
          </div>
          <button
            onClick={() => setShowSimpleView(!showSimpleView)}
            className="px-3 py-1 text-xs bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-md transition-colors"
          >
            {showSimpleView ? "Advanced View" : "Simple View"}
          </button>
        </div>
      </div>

      {/* Content */}
      {showSimpleView ? (
        // Simple view - just the ModelSelector
        <div className="flex-1 overflow-y-auto p-6">
          <div className="max-w-4xl mx-auto">
            <ModelSelector />
          </div>
        </div>
      ) : (
        // Advanced view - Two Panel Layout (for future enhancement)
        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Models List */}
          <div className="w-80 border-r border-gray-800 overflow-y-auto bg-gray-900/50">
            <ModelsList
              selectedId={selectedModelId}
              onSelect={setSelectedModelId}
            />
          </div>

          {/* Right Panel - Model Detail */}
          <div className="flex-1 overflow-y-auto">
            <ModelDetail modelId={selectedModelId} />
          </div>
        </div>
      )}
    </div>
  );
}
