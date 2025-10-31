import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Mic, Package, Activity, Zap, Circle } from "lucide-react";
import { LogsList } from "../logs/LogsList";
import { LogsDetail } from "../logs/LogsDetail";
import { PageHeader } from "../shared";
import { useSettings } from "../../hooks/useSettings";
import { useWorkflows } from "../../hooks/useWorkflows";

type StatusType = "idle" | "active" | "recording" | "error" | "loading";

interface StatusCardProps {
  icon: React.ElementType;
  title: string;
  value: string;
  status?: StatusType;
}

function StatusCard({ icon: Icon, title, value, status = "idle" }: StatusCardProps) {
  // Explicit color maps (no string parsing)
  const statusStyles: Record<StatusType, {
    iconColor: string;
    textColor: string;
    borderColor: string;
    bgColor: string;
  }> = {
    idle: {
      iconColor: "text-gray-400",
      textColor: "text-gray-300",
      borderColor: "border-gray-800",
      bgColor: "bg-gray-900/50",
    },
    active: {
      iconColor: "uw-text-success",
      textColor: "uw-text-success",
      borderColor: "uw-border-success",
      bgColor: "uw-bg-success-dim",
    },
    recording: {
      iconColor: "uw-text-error",
      textColor: "uw-text-error",
      borderColor: "uw-border-error",
      bgColor: "uw-bg-error-dim",
    },
    error: {
      iconColor: "uw-text-error",
      textColor: "uw-text-error",
      borderColor: "uw-border-error",
      bgColor: "uw-bg-error-dim",
    },
    loading: {
      iconColor: "uw-text-warning",
      textColor: "uw-text-warning",
      borderColor: "uw-border-warning",
      bgColor: "uw-bg-warning-dim",
    },
  };

  const styles = statusStyles[status];

  return (
    <div className={`p-4 rounded-lg border ${styles.borderColor} ${styles.bgColor} transition-all duration-200`}>
      <div className="flex items-center gap-3">
        <Icon className={`w-5 h-5 ${styles.iconColor}`} />
        <div className="flex-1 min-w-0">
          <p className="text-xs uw-text-secondary uppercase tracking-wider">{title}</p>
          <p className={`text-sm font-medium mt-1 truncate ${styles.textColor}`}>
            {value}
          </p>
        </div>
        {(status === "active" || status === "recording") && (
          <Circle className={`w-2 h-2 ${styles.iconColor} animate-pulse`} fill="currentColor" />
        )}
      </div>
    </div>
  );
}

export function SystemMonitorPage() {
  const [selectedLogId, setSelectedLogId] = useState<string | null>("application-logs");
  const [isRecording, setIsRecording] = useState(false);
  const [modelStatus, setModelStatus] = useState<string>("Ready");
  const [modelName, setModelName] = useState<string>("");
  const [cachedModelName, setCachedModelName] = useState<string>("");
  const [activeWorkflowId, setActiveWorkflowId] = useState<string | null>(null);
  const { settings } = useSettings();
  const { workflows } = useWorkflows();

  // Fetch model info with correct param name
  useEffect(() => {
    const fetchModelInfo = async () => {
      try {
        let modelId = settings?.selected_model;

        // Fallback: get current model if no selected_model
        if (!modelId) {
          modelId = await invoke<string>("get_current_model");
        }

        if (modelId) {
          const info = await invoke<{ name: string }>("get_model_info", {
            // Tauri expects `model_id`; include both to be robust across bindings
            modelId,
            model_id: modelId,
          });

          if (info?.name) {
            setModelName(info.name);
            setCachedModelName(info.name); // Cache to avoid flicker
          }
        }
      } catch (error) {
        console.error("Failed to fetch model info:", error);
        // Keep using cached name if fetch fails
        if (cachedModelName) {
          setModelName(cachedModelName);
        }
      }
    };

    if (modelStatus === "Ready") {
      fetchModelInfo();
    }
  }, [modelStatus, settings?.selected_model, cachedModelName]);

  // Resolve microphone label
  const getMicrophoneLabel = (): string => {
    const selected = settings?.selected_microphone || "Default";

    if (selected === "Default" || selected === "default") {
      // Best-effort: try to get actual default device name
      // This would require accessing the audio devices list
      return "System Default";
    }

    return selected;
  };

  // Listen for events to show status
  useEffect(() => {
    let micLevelTimeout: ReturnType<typeof setTimeout> | null = null;
    const cleanups: UnlistenFn[] = [];

    (async () => {
      // Recording status - use mic-level events as a proxy
      const unlistenMicLevel = await listen("mic-level", () => {
        setIsRecording(true);
        if (micLevelTimeout) clearTimeout(micLevelTimeout);
        micLevelTimeout = setTimeout(() => setIsRecording(false), 500);
      });
      cleanups.push(unlistenMicLevel);

      // Model status
      const unlistenModelState = await listen<{ event_type: string }>(
        "model-state-changed",
        (event) => {
          const t = event.payload?.event_type;
          const mapped =
            t === "loading_started"
              ? "Loading"
              : t === "loading_completed"
              ? "Ready"
              : t === "unloaded"
              ? "Unloaded"
              : t === "loading_failed"
              ? "Error"
              : "Unknown";
          setModelStatus(mapped);
        }
      );
      cleanups.push(unlistenModelState);

      // Workflow recording events
      const unlistenWorkflowStarted = await listen<{ workflow_id: string }>(
        "workflow-recording-started",
        (event) => setActiveWorkflowId(event.payload.workflow_id)
      );
      cleanups.push(unlistenWorkflowStarted);

      const unlistenWorkflowStopped = await listen<{ workflow_id: string }>(
        "workflow-recording-stopped",
        () => setActiveWorkflowId(null)
      );
      cleanups.push(unlistenWorkflowStopped);
    })();

    return () => {
      cleanups.forEach((fn) => fn());
      if (micLevelTimeout) clearTimeout(micLevelTimeout);
    };
  }, []);

  // Determine status for each card
  const getModelCardStatus = (): StatusType => {
    if (modelStatus === "Loading") return "loading";
    if (modelStatus === "Error") return "error";
    if (modelStatus === "Ready") return "active";
    return "idle";
  };

  const getMicrophoneStatus = (): StatusType => {
    if (settings?.always_on_microphone) return "active";
    return settings?.selected_microphone ? "idle" : "error";
  };

  // Get model display value
  const getModelValue = (): string => {
    if (modelStatus === "Ready" && modelName) {
      return modelName;
    }
    return modelStatus;
  };

  return (
    <div className="flex-1 flex flex-col h-full uw-bg-surface">
      {/* Header */}
      <PageHeader
        title="System Monitor"
        subtitle="Real-time application status and logs"
      />

      {/* Stats Bar */}
      <div className="px-6 py-4 border-b uw-border-default uw-bg-card">
        <div className="flex items-center gap-8">
          <div className="flex flex-col gap-1">
            <span className="text-xs uw-text-secondary uppercase tracking-wider">Status</span>
            <span className={`uw-mono text-lg font-semibold ${isRecording ? "uw-text-error" : "uw-text-accent"}`}>
              {isRecording ? "RECORDING" : "IDLE"}
            </span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs uw-text-secondary uppercase tracking-wider">Model</span>
            <span className="uw-mono text-sm font-medium uw-text-primary">
              {modelName || modelStatus}
            </span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs uw-text-secondary uppercase tracking-wider">Mic</span>
            <span className="uw-mono text-sm font-medium uw-text-primary">
              {getMicrophoneLabel()}
            </span>
          </div>
          {isRecording && activeWorkflowId && (() => {
            const activeWorkflow = workflows.find(w => w.id === activeWorkflowId);
            if (activeWorkflow) {
              const hotkey = activeWorkflow.trigger?.type === "Hotkey" ? activeWorkflow.trigger.binding : "";
              return (
                <div className="flex flex-col gap-1">
                  <span className="text-xs uw-text-secondary uppercase tracking-wider">Workflow</span>
                  <span className="uw-mono text-sm font-medium uw-text-primary">
                    {activeWorkflow.name}{hotkey ? ` • ${hotkey}` : ""}
                  </span>
                </div>
              );
            }
            return null;
          })()}
        </div>
      </div>

      {/* Status Cards Grid */}
      <div className="px-6 py-4 border-b uw-border-default uw-bg-card">
        <div className="grid grid-cols-4 gap-4">
          <StatusCard
            icon={Activity}
            title="Recording"
            value={isRecording ? "Active" : "Idle"}
            status={isRecording ? "recording" : "idle"}
          />
          <StatusCard
            icon={Package}
            title="Model"
            value={getModelValue()}
            status={getModelCardStatus()}
          />
          <StatusCard
            icon={Mic}
            title="Microphone"
            value={getMicrophoneLabel()}
            status={getMicrophoneStatus()}
          />
          <StatusCard
            icon={Zap}
            title="Workflows"
            value={`${workflows.length} Configured`}
            status="idle"
          />
        </div>
      </div>

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Logs List */}
          <div className="w-80 border-r uw-border-default overflow-y-auto uw-scroll uw-bg-elevated">
            <LogsList
              selectedId={selectedLogId}
              onSelect={setSelectedLogId}
            />
          </div>

          {/* Right Panel - Log Details */}
          <div className="flex-1 overflow-y-auto uw-scroll">
            <LogsDetail logId={selectedLogId} />
          </div>
      </div>
    </div>
  );
}
