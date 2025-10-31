import React from "react";
import { PrimaryButton } from "../shared";
import { useModels } from "../../hooks/useModels";
import { Download, CheckCircle } from "lucide-react";

interface ModelsListProps {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export const ModelsList: React.FC<ModelsListProps> = ({
  selectedId,
  onSelect,
}) => {
  const { models, currentModel, loading } = useModels();

  const handleNewModel = () => {
    // Placeholder - will open model download/management
    console.log("New model clicked");
  };

  if (loading) {
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b uw-border-default">
          <h2 className="text-lg font-semibold uw-text-primary">Models</h2>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="uw-text-secondary">Loading models...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b uw-border-default">
        <h2 className="text-lg font-semibold uw-text-primary">Models</h2>
        <PrimaryButton onClick={handleNewModel} fullWidth className="mt-3">
          + New Model
        </PrimaryButton>
      </div>
      <div className="flex-1 overflow-y-auto uw-scroll p-2">
        {models.map((model) => {
          const isActive = selectedId === model.id;
          const isCurrentModel = currentModel === model.id;
          const isDownloaded = model.is_downloaded;

          return (
            <div
              key={model.id}
              className={`
                p-3 rounded-lg cursor-pointer transition-all duration-150 mb-2
                border
                ${isActive
                  ? "uw-bg-primary-dim uw-border-primary uw-text-accent"
                  : "hover:uw-bg-card hover:border-gray-700 uw-text-primary border-transparent"
                }
              `}
              onClick={() => onSelect(model.id)}
            >
              <div className="flex items-start gap-2">
                {isDownloaded ? (
                  <CheckCircle className={`w-4 h-4 flex-shrink-0 mt-0.5 ${isActive ? "uw-text-accent" : "text-green-500"}`} />
                ) : (
                  <Download className={`w-4 h-4 flex-shrink-0 mt-0.5 ${isActive ? "uw-text-accent" : "text-gray-500"}`} />
                )}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    {isCurrentModel && (
                      <span className="px-2 py-0.5 text-xs rounded-full bg-green-500/10 text-green-500 border border-green-500/20">
                        Active
                      </span>
                    )}
                    {isDownloaded && !isCurrentModel && (
                      <span className="px-2 py-0.5 text-xs rounded-full uw-bg-primary-dim uw-text-accent uw-border-primary border">
                        Downloaded
                      </span>
                    )}
                  </div>
                  <p className="text-sm font-medium truncate">{model.name}</p>
                  <p className="text-xs uw-text-secondary mt-1">
                    {model.is_directory ? "Local Model" : "Cloud API"} • {Math.round(model.size_mb)} MB
                  </p>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
