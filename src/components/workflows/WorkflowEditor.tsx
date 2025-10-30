import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { StoredWorkflow, ModelInfo, TriggerConfig } from "../../lib/types";
import { Destination } from "../../lib/types";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";

interface WorkflowEditorProps {
  workflowId: string;
}

export const WorkflowEditor: React.FC<WorkflowEditorProps> = ({
  workflowId,
}) => {
  // Form state
  const [formData, setFormData] = useState<StoredWorkflow | null>(null);
  const [originalData, setOriginalData] = useState<StoredWorkflow | null>(null);

  // UI state
  const [isDirty, setIsDirty] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [validationErrors, setValidationErrors] = useState<
    Record<string, string>
  >({});

  // Data dependencies
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [destinations, setDestinations] = useState<Destination[]>([]);
  const [isExisting, setIsExisting] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  const { settings } = useSettings();

  // Load workflow and dependencies on mount
  const loadWorkflow = useCallback(async () => {
    try {
      setIsLoading(true);

      // Load workflow
      const workflow = await invoke<StoredWorkflow | null>("get_workflow", {
        id: workflowId,
      });

      if (workflow) {
        // Existing workflow
        setFormData(workflow);
        setOriginalData(structuredClone(workflow));
        setIsExisting(true);
      } else {
        // New workflow - initialize with defaults
        const newWorkflow: StoredWorkflow = {
          id: workflowId,
          name: "",
          enabled: true,
          trigger: { type: "Hotkey", binding: "", push_to_talk: false },
          model: {
            model_id: settings?.selected_model || "",
            language: settings?.selected_language || "auto",
            translate_to_english: settings?.translate_to_english || false,
          },
          destination_ids: [],
          notes: undefined,
        };
        setFormData(newWorkflow);
        setOriginalData(structuredClone(newWorkflow));
        setIsExisting(false);
      }

      // Load models
      const availableModels = await invoke<ModelInfo[]>(
        "get_available_models"
      );
      setModels(availableModels);

      // Load destinations
      const availableDestinations = await invoke<Destination[]>(
        "list_destinations"
      );
      setDestinations(availableDestinations);
    } catch (err) {
      console.error("Failed to load workflow:", err);
      toast.error("Failed to load workflow");
    } finally {
      setIsLoading(false);
    }
  }, [workflowId, settings]);

  useEffect(() => {
    loadWorkflow();
  }, [loadWorkflow]);

  // Dirty detection
  useEffect(() => {
    if (!formData || !originalData) {
      setIsDirty(false);
      return;
    }

    const isDifferent =
      JSON.stringify(formData) !== JSON.stringify(originalData);
    setIsDirty(isDifferent);
  }, [formData, originalData]);

  // Handle external changes (workflows-changed event)
  useEffect(() => {
    const setupListener = async () => {
      const unlisten = await listen("workflows-changed", () => {
        if (isDirty) {
          // Don't auto-reload, show toast instead
          toast.info("Workflow changed in background", {
            action: {
              label: "Refresh",
              onClick: () => {
                loadWorkflow();
              },
            },
          });
        } else {
          // No unsaved changes, safe to reload
          loadWorkflow();
        }
      });

      return unlisten;
    };

    let unlisten: (() => void) | undefined;
    setupListener().then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [isDirty, loadWorkflow]);

  // Normalization helpers
  const normalizeWorkflow = (workflow: StoredWorkflow): StoredWorkflow => {
    const normalized = { ...workflow };

    // Normalize name
    normalized.name = workflow.name.trim();

    // Normalize trigger
    if (workflow.trigger.type === "Hotkey") {
      // Normalize binding: lowercase, trim spaces around +
      normalized.trigger = {
        ...workflow.trigger,
        binding: workflow.trigger.binding
          .split("+")
          .map((s) => s.trim().toLowerCase())
          .join("+"),
      };
    }

    if (workflow.trigger.type === "FolderWatch") {
      // Normalize paths: trim, filter empty
      const paths = workflow.trigger.paths
        .map((p) => p.trim())
        .filter((p) => p.length > 0);

      // Normalize patterns: ensure "*.ext" format, dedupe
      const patterns = workflow.trigger.file_patterns
        .map((p) => {
          const trimmed = p.trim();
          // Convert ".wav" → "*.wav"
          return trimmed.startsWith("*.")
            ? trimmed
            : `*.${trimmed.replace(/^\*?\.?/, "")}`;
        })
        .filter((p, i, arr) => arr.indexOf(p) === i); // Dedupe

      normalized.trigger = {
        ...workflow.trigger,
        paths,
        file_patterns: patterns,
      };
    }

    // Dedupe destination IDs
    normalized.destination_ids = [...new Set(workflow.destination_ids)];

    return normalized;
  };

  // Validation
  const validateWorkflow = (
    workflow: StoredWorkflow
  ): Record<string, string> => {
    const errors: Record<string, string> = {};

    // Name
    if (!workflow.name.trim()) {
      errors.name = "Name is required";
    }

    // Trigger
    if (workflow.trigger.type === "Hotkey") {
      if (!workflow.trigger.binding.trim()) {
        errors.binding = "Hotkey binding is required";
      }
    }

    if (workflow.trigger.type === "FolderWatch") {
      if (workflow.trigger.paths.length === 0) {
        errors.paths = "At least one path is required";
      }

      if (workflow.trigger.file_patterns.length === 0) {
        errors.patterns = "At least one file pattern is required";
      }

      if (workflow.trigger.interval_seconds < 10) {
        errors.interval = "Interval must be at least 10 seconds";
      }

      if (workflow.trigger.stability_timeout_seconds < 1) {
        errors.stability = "Stability timeout must be at least 1 second";
      }
    }

    // Model
    if (!workflow.model.model_id.trim()) {
      errors.model = "Model selection is required";
    }

    return errors;
  };

  // Backend error mapping
  const mapBackendError = (errorMessage: string): Record<string, string> => {
    const errors: Record<string, string> = {};

    if (errorMessage.includes("name cannot be empty")) {
      errors.name = "Name is required";
    }

    if (errorMessage.includes("binding cannot be empty")) {
      errors.binding = "Hotkey binding is required";
    }

    if (errorMessage.includes("at least one path")) {
      errors.paths = "At least one folder path is required";
    }

    if (errorMessage.includes("at least one file pattern")) {
      errors.patterns = "At least one file pattern is required";
    }

    if (errorMessage.includes("interval must be at least")) {
      errors.interval = "Check interval must be at least 10 seconds";
    }

    if (errorMessage.includes("stability timeout")) {
      errors.stability = "Stability timeout must be at least 1 second";
    }

    if (
      errorMessage.includes("destination") &&
      errorMessage.includes("does not exist")
    ) {
      errors.destinations =
        "One or more selected destinations no longer exist";
    }

    // If no specific field matched, show as general error
    if (Object.keys(errors).length === 0) {
      errors.general = errorMessage;
    }

    return errors;
  };

  // Form update helpers
  const updateField = <K extends keyof StoredWorkflow>(
    field: K,
    value: StoredWorkflow[K]
  ) => {
    if (!formData) return;
    setFormData({ ...formData, [field]: value });
  };

  const updateTriggerType = (type: "Hotkey" | "FolderWatch") => {
    if (!formData) return;

    let newTrigger: TriggerConfig;

    if (type === "Hotkey") {
      newTrigger = {
        type: "Hotkey",
        binding: "",
        push_to_talk: false,
      };
    } else {
      newTrigger = {
        type: "FolderWatch",
        paths: [],
        file_patterns: ["*.wav"],
        interval_seconds: 60,
        stability_timeout_seconds: 30,
      };
    }

    setFormData({ ...formData, trigger: newTrigger });
  };

  const updateTriggerField = (field: string, value: any) => {
    if (!formData) return;
    setFormData({
      ...formData,
      trigger: { ...formData.trigger, [field]: value },
    });
  };

  const updateModelField = (field: string, value: any) => {
    if (!formData) return;
    setFormData({
      ...formData,
      model: { ...formData.model, [field]: value },
    });
  };

  const toggleDestination = (id: string, checked: boolean) => {
    if (!formData) return;

    const newIds = checked
      ? [...formData.destination_ids, id]
      : formData.destination_ids.filter((destId) => destId !== id);

    setFormData({ ...formData, destination_ids: newIds });
  };

  // Parse comma-separated file patterns
  const parsePatterns = (value: string): string[] => {
    return value
      .split(",")
      .map((p) => p.trim())
      .filter((p) => p.length > 0);
  };

  // Actions
  const handleSave = async () => {
    if (!formData) return;

    // Clear previous errors
    setValidationErrors({});

    // Client-side validation
    const clientErrors = validateWorkflow(formData);
    if (Object.keys(clientErrors).some((k) => clientErrors[k])) {
      setValidationErrors(clientErrors);
      toast.error("Please fix validation errors");
      return;
    }

    // Normalize before saving
    const normalized = normalizeWorkflow(formData);

    try {
      setIsSaving(true);

      await invoke("upsert_workflow", { workflow: normalized });

      // Update original data to match saved data
      setOriginalData(structuredClone(normalized));
      setFormData(normalized);
      setIsExisting(true);

      toast.success("Workflow saved successfully");

      // workflows-changed event will fire automatically from backend
    } catch (err) {
      console.error("Failed to save workflow:", err);

      // Map backend error to field-specific errors
      const backendErrors = mapBackendError(
        err instanceof Error ? err.message : String(err)
      );
      setValidationErrors(backendErrors);

      toast.error("Failed to save workflow");
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!isExisting || !formData) return;

    const confirmed = window.confirm(
      `Are you sure you want to delete "${formData.name}"? This cannot be undone.`
    );

    if (!confirmed) return;

    try {
      setIsDeleting(true);

      await invoke("delete_workflow", { id: workflowId });

      toast.success("Workflow deleted");

      // Parent will handle navigation via workflows-changed event
    } catch (err) {
      console.error("Failed to delete workflow:", err);
      toast.error("Failed to delete workflow");
    } finally {
      setIsDeleting(false);
    }
  };

  const handleDiscard = () => {
    if (!isDirty || !originalData) return;

    // Reset to original data
    setFormData(structuredClone(originalData));
    setValidationErrors({});
    toast.info("Changes discarded");
  };

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl+S to save
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }

      // Escape to discard
      if (e.key === "Escape" && isDirty) {
        handleDiscard();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isDirty, formData, originalData]);

  // Get destination type label
  const getTypeLabel = (dest: Destination): string => {
    const config = dest.config;
    if ("ActiveWindow" in config) return "Active Window";
    if ("FileSystem" in config) return "File System";
    if ("Telegram" in config) return "Telegram";
    return "Unknown";
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-mid-gray">Loading workflow...</p>
      </div>
    );
  }

  if (!formData) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-mid-gray">Failed to load workflow</p>
      </div>
    );
  }

  const downloadedModels = models.filter((m) => m.is_downloaded);
  const currentModelInList =
    downloadedModels.find((m) => m.id === formData.model.model_id) ||
    models.find((m) => m.id === formData.model.model_id);

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* General Section */}
        <SettingsGroup title="General">
          <SettingContainer
            label="Name"
            description="Descriptive name for this workflow"
            error={validationErrors.name}
          >
            <Input
              value={formData.name}
              onChange={(e) => updateField("name", e.target.value)}
              placeholder="Quick Capture"
              className={validationErrors.name ? "border-red-500" : ""}
            />
          </SettingContainer>

          <SettingContainer
            label="Enabled"
            description="Workflow is active and can be triggered"
          >
            <ToggleSwitch
              checked={formData.enabled}
              onCheckedChange={(checked) => updateField("enabled", checked)}
            />
          </SettingContainer>

          <SettingContainer
            label="Notes"
            description="Optional description or notes"
          >
            <textarea
              value={formData.notes || ""}
              onChange={(e) => updateField("notes", e.target.value || undefined)}
              rows={3}
              placeholder="Additional notes about this workflow..."
              className="w-full p-2 rounded border border-mid-gray/20 bg-background text-foreground resize-none focus:outline-none focus:ring-2 focus:ring-logo-primary"
            />
          </SettingContainer>
        </SettingsGroup>

        {/* Trigger Section */}
        <SettingsGroup title="Trigger">
          <SettingContainer
            label="Type"
            description="How this workflow is triggered"
          >
            <div className="flex gap-4">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  checked={formData.trigger.type === "Hotkey"}
                  onChange={() => updateTriggerType("Hotkey")}
                  className="cursor-pointer"
                />
                <span>Hotkey</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  checked={formData.trigger.type === "FolderWatch"}
                  onChange={() => updateTriggerType("FolderWatch")}
                  className="cursor-pointer"
                />
                <span>Folder Watch</span>
              </label>
            </div>
          </SettingContainer>

          {formData.trigger.type === "Hotkey" && (
            <>
              <SettingContainer
                label="Binding"
                description="Keyboard shortcut (e.g., cmd+shift+s)"
                error={validationErrors.binding}
              >
                <Input
                  value={formData.trigger.binding}
                  onChange={(e) => updateTriggerField("binding", e.target.value)}
                  placeholder="cmd+shift+s"
                  className={validationErrors.binding ? "border-red-500" : ""}
                />
              </SettingContainer>

              <SettingContainer
                label="Push to Talk"
                description="Hold key to record, release to stop"
              >
                <ToggleSwitch
                  checked={formData.trigger.push_to_talk}
                  onCheckedChange={(checked) =>
                    updateTriggerField("push_to_talk", checked)
                  }
                />
              </SettingContainer>
            </>
          )}

          {formData.trigger.type === "FolderWatch" && (
            <>
              <SettingContainer
                label="Watch Folders"
                description="Paths to monitor for new audio files"
                error={validationErrors.paths}
              >
                <div className="space-y-2">
                  {formData.trigger.paths.map((path, index) => (
                    <div key={index} className="flex gap-2">
                      <Input
                        value={path}
                        onChange={(e) => {
                          const newPaths = [...formData.trigger.paths];
                          newPaths[index] = e.target.value;
                          updateTriggerField("paths", newPaths);
                        }}
                        placeholder="/path/to/folder"
                        className="flex-1"
                      />
                      <Button
                        variant="secondary"
                        onClick={() => {
                          const newPaths = formData.trigger.paths.filter(
                            (_, i) => i !== index
                          );
                          updateTriggerField("paths", newPaths);
                        }}
                      >
                        Remove
                      </Button>
                    </div>
                  ))}
                  <Button
                    variant="secondary"
                    onClick={() => {
                      updateTriggerField("paths", [
                        ...formData.trigger.paths,
                        "",
                      ]);
                    }}
                  >
                    + Add Path
                  </Button>
                </div>
              </SettingContainer>

              <SettingContainer
                label="File Patterns"
                description="Comma-separated (e.g., *.wav, *.mp3)"
                error={validationErrors.patterns}
              >
                <Input
                  value={formData.trigger.file_patterns.join(", ")}
                  onChange={(e) =>
                    updateTriggerField("file_patterns", parsePatterns(e.target.value))
                  }
                  placeholder="*.wav, *.mp3, *.m4a"
                  className={validationErrors.patterns ? "border-red-500" : ""}
                />
              </SettingContainer>

              <SettingContainer
                label="Check Interval"
                description="Seconds between checks (min: 10)"
                error={validationErrors.interval}
              >
                <Input
                  type="number"
                  min={10}
                  value={formData.trigger.interval_seconds}
                  onChange={(e) =>
                    updateTriggerField(
                      "interval_seconds",
                      parseInt(e.target.value) || 10
                    )
                  }
                  className={validationErrors.interval ? "border-red-500" : ""}
                />
              </SettingContainer>

              <SettingContainer
                label="Stability Timeout"
                description="Seconds to wait for file to stabilize (min: 1)"
                error={validationErrors.stability}
              >
                <Input
                  type="number"
                  min={1}
                  value={formData.trigger.stability_timeout_seconds}
                  onChange={(e) =>
                    updateTriggerField(
                      "stability_timeout_seconds",
                      parseInt(e.target.value) || 1
                    )
                  }
                  className={validationErrors.stability ? "border-red-500" : ""}
                />
              </SettingContainer>
            </>
          )}
        </SettingsGroup>

        {/* Model Section */}
        <SettingsGroup title="Model">
          <SettingContainer
            label="Model"
            description="Whisper model for transcription"
            error={validationErrors.model}
          >
            {downloadedModels.length === 0 && !currentModelInList ? (
              <div className="text-sm text-mid-gray">
                No models available.{" "}
                <button className="text-logo-primary hover:underline">
                  Manage Models →
                </button>
              </div>
            ) : (
              <select
                value={formData.model.model_id}
                onChange={(e) => updateModelField("model_id", e.target.value)}
                className="w-full p-2 rounded border border-mid-gray/20 bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-logo-primary"
              >
                {downloadedModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name}
                  </option>
                ))}
                {currentModelInList &&
                  !currentModelInList.is_downloaded && (
                    <option value={currentModelInList.id}>
                      {currentModelInList.name} (Not downloaded)
                    </option>
                  )}
              </select>
            )}
          </SettingContainer>

          <SettingContainer
            label="Language"
            description="Input language for transcription"
          >
            <select
              value={formData.model.language}
              onChange={(e) => updateModelField("language", e.target.value)}
              className="w-full p-2 rounded border border-mid-gray/20 bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-logo-primary"
            >
              <option value="auto">Auto-detect</option>
              <option value="en">English</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
              <option value="it">Italian</option>
              <option value="pt">Portuguese</option>
              <option value="zh">Chinese</option>
              <option value="ja">Japanese</option>
            </select>
          </SettingContainer>

          <SettingContainer
            label="Translate to English"
            description="Force output to English"
          >
            <ToggleSwitch
              checked={formData.model.translate_to_english}
              onCheckedChange={(checked) =>
                updateModelField("translate_to_english", checked)
              }
            />
          </SettingContainer>
        </SettingsGroup>

        {/* Destinations Section */}
        <SettingsGroup title="Destinations">
          <SettingContainer
            label="Output Destinations"
            description="Where to send transcribed text"
            error={validationErrors.destinations}
          >
            {destinations.length === 0 ? (
              <div className="text-sm text-mid-gray">
                No destinations available.{" "}
                <button className="text-logo-primary hover:underline">
                  Open Destinations →
                </button>
              </div>
            ) : (
              <div className="space-y-2">
                {destinations.map((dest) => (
                  <label
                    key={dest.id}
                    className="flex items-center gap-2 p-2 hover:bg-mid-gray/10 rounded cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={formData.destination_ids.includes(dest.id)}
                      onChange={(e) =>
                        toggleDestination(dest.id, e.target.checked)
                      }
                      className="cursor-pointer"
                    />
                    <span className="flex-1">{dest.name}</span>
                    <span className="text-xs px-2 py-1 bg-mid-gray/20 rounded">
                      {getTypeLabel(dest)}
                    </span>
                  </label>
                ))}
              </div>
            )}
          </SettingContainer>

          {formData.destination_ids.length === 0 && destinations.length > 0 && (
            <div className="text-sm text-yellow-600 dark:text-yellow-400">
              ⚠️ At least one destination recommended
            </div>
          )}
        </SettingsGroup>
      </div>

      {/* Actions Bar */}
      <div className="sticky bottom-0 border-t border-mid-gray/20 bg-background p-4 flex gap-3 items-center">
        {validationErrors.general && (
          <div className="flex-1 text-sm text-red-600 dark:text-red-400">
            {validationErrors.general}
          </div>
        )}

        {isDirty && !validationErrors.general && (
          <div className="flex-1 text-sm text-mid-gray">Unsaved changes</div>
        )}

        <div className="flex gap-3 ml-auto">
          {isExisting && (
            <Button
              variant="secondary"
              onClick={handleDelete}
              disabled={isDeleting || isSaving}
              className="bg-red-500/20 hover:bg-red-500/30 text-red-600 dark:text-red-400"
            >
              {isDeleting ? "Deleting..." : "Delete"}
            </Button>
          )}

          <Button
            variant="secondary"
            onClick={handleDiscard}
            disabled={!isDirty || isSaving || isDeleting}
          >
            Discard
          </Button>

          <Button
            onClick={handleSave}
            disabled={
              isSaving || isDeleting || downloadedModels.length === 0
            }
          >
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </div>
      </div>
    </div>
  );
};
