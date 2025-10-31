import React from "react";
import { useModels } from "../../hooks/useModels";
import { ConfigCard, ConfigField } from "../shared/ConfigCard";
import { PrimaryButton } from "../shared";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Dropdown } from "../ui/Dropdown";

interface ModelDetailProps {
  modelId: string | null;
}

export const ModelDetail: React.FC<ModelDetailProps> = ({ modelId }) => {
  const {
    models,
    currentModel,
    selectModel,
    downloadModel,
    deleteModel,
    isModelDownloading,
    getDownloadProgress,
  } = useModels();

  const model = models.find((m) => m.id === modelId);

  if (!modelId || !model) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Select a model to view details</p>
      </div>
    );
  }

  const isActive = currentModel === model.id;
  const isDownloaded = model.is_downloaded;
  const isDownloading = isModelDownloading(model.id);
  const downloadProgress = getDownloadProgress(model.id);

  const handleSetAsDefault = async () => {
    if (isDownloaded) {
      await selectModel(model.id);
    }
  };

  const handleDownload = async () => {
    await downloadModel(model.id);
  };

  const handleDelete = async () => {
    if (confirm(`Are you sure you want to delete ${model.name}?`)) {
      await deleteModel(model.id);
    }
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-semibold uw-text-primary">{model.name}</h1>
        <p className="text-sm uw-text-secondary mt-1">{model.description}</p>
      </div>

      {/* Model Information */}
      <ConfigCard title="Model Information">
        <ConfigField label="Model Type">
          <Input
            type="text"
            value={model.is_directory ? "Local (GGML)" : "Cloud API"}
            disabled
            className="uw-bg-card"
          />
        </ConfigField>

        <ConfigField label="Size on Disk">
          <Input
            type="text"
            value={`${Math.round(model.size_mb)} MB`}
            disabled
            className="uw-bg-card"
          />
        </ConfigField>

        <ConfigField
          label="RAM Usage"
          hint="Approximate memory usage when loaded"
        >
          <Input
            type="text"
            value={`~${Math.round(model.size_mb * 1.5)} MB`}
            disabled
            className="uw-bg-card"
          />
        </ConfigField>

        <ConfigField label="Status">
          <div className="flex items-center gap-2">
            {isActive ? (
              <span className="px-3 py-1.5 text-sm rounded-md bg-green-500/10 text-green-500 border border-green-500/20">
                Active
              </span>
            ) : isDownloaded ? (
              <span className="px-3 py-1.5 text-sm rounded-md uw-bg-primary-dim uw-text-accent uw-border-primary border">
                Downloaded
              </span>
            ) : isDownloading ? (
              <span className="px-3 py-1.5 text-sm rounded-md bg-blue-500/10 text-blue-500 border border-blue-500/20">
                Downloading {downloadProgress?.percentage.toFixed(0)}%
              </span>
            ) : (
              <span className="px-3 py-1.5 text-sm rounded-md bg-gray-500/10 text-gray-500 border border-gray-500/20">
                Not Downloaded
              </span>
            )}
          </div>
        </ConfigField>
      </ConfigCard>

      {/* Performance Settings (only for downloaded models) */}
      {isDownloaded && (
        <ConfigCard title="Performance Settings">
          <ConfigField
            label="Processing Threads"
            hint="Number of CPU threads to use for inference"
          >
            <Input type="number" defaultValue="4" />
          </ConfigField>

          <ConfigField label="GPU Acceleration">
            <Dropdown
              selectedValue="auto"
              onSelect={() => {}}
              options={[
                { value: "auto", label: "Auto-detect" },
                { value: "metal", label: "Metal (macOS)" },
                { value: "cuda", label: "CUDA" },
                { value: "cpu", label: "CPU Only" },
              ]}
            />
          </ConfigField>
        </ConfigCard>
      )}

      {/* Actions */}
      <div className="flex gap-3">
        {isDownloaded ? (
          <>
            {!isActive && (
              <PrimaryButton onClick={handleSetAsDefault}>
                Set as Default
              </PrimaryButton>
            )}
            <Button variant="secondary" size="md">
              Test Model
            </Button>
            <Button variant="danger" size="md" onClick={handleDelete}>
              Delete Model
            </Button>
          </>
        ) : (
          <PrimaryButton onClick={handleDownload} disabled={isDownloading}>
            {isDownloading ? `Downloading ${downloadProgress?.percentage.toFixed(0)}%` : "Download Model"}
          </PrimaryButton>
        )}
      </div>
    </div>
  );
};
