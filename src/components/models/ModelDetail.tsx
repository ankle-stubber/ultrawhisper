import React from "react";
import ModelSelector from "../model-selector/ModelSelector";

interface ModelDetailProps {
  modelId: string | null;
}

export const ModelDetail: React.FC<ModelDetailProps> = ({ modelId }) => {
  if (!modelId) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Select model management</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="flex flex-col items-center p-4 gap-4">
        <ModelSelector />
      </div>
    </div>
  );
};
