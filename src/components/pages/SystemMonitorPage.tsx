import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Mic, Package, Activity, Shield, Circle } from "lucide-react";
import { LogsList } from "../logs/LogsList";
import { LogsDetail } from "../logs/LogsDetail";
import { useSettings } from "../../hooks/useSettings";

interface StatusCardProps {
  icon: React.ElementType;
  title: string;
  value: string;
  status?: "idle" | "active" | "error" | "loading";
}

function StatusCard({ icon: Icon, title, value, status = "idle" }: StatusCardProps) {
  const statusColors = {
    idle: "text-gray-400 border-gray-800",
    active: "text-green-500 border-green-500/20",
    error: "text-red-500 border-red-500/20",
    loading: "text-amber-500 border-amber-500/20",
  } as const;

  const bgColors = {
    idle: "bg-gray-900/50",
    active: "bg-green-500/5",
    error: "bg-red-500/5",
    loading: "bg-amber-500/5",
  } as const;

  return (
    <div className={`p-4 rounded-lg border ${statusColors[status]} ${bgColors[status]} transition-all duration-200`}>
      <div className="flex items-center gap-3">
        <Icon className={`w-5 h-5 ${statusColors[status].split(' ')[0]}`} />
        <div className="flex-1 min-w-0">
          <p className="text-xs text-gray-500 uppercase tracking-wider">{title}</p>
          <p className={`text-sm font-medium mt-1 truncate ${status === "idle" ? "text-gray-300" : ""}`}>
            {value}
          </p>
        </div>
        {status === "active" && (
          <Circle className="w-2 h-2 text-green-500 animate-pulse" fill="currentColor" />
        )}
      </div>
    </div>
  );
}

export function SystemMonitorPage() {
  const [selectedLogId, setSelectedLogId] = useState<string | null>("application-logs");
  const [isRecording, setIsRecording] = useState(false);
  const [modelStatus, setModelStatus] = useState<string>("Ready");
  const { settings } = useSettings();

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
    })();

    return () => {
      cleanups.forEach((fn) => fn());
      if (micLevelTimeout) clearTimeout(micLevelTimeout);
    };
  }, []);

  // Determine status for each card
  const getModelCardStatus = () => {
    if (modelStatus === "Loading") return "loading";
    if (modelStatus === "Error") return "error";
    if (modelStatus === "Ready") return "active";
    return "idle";
  };

  const getMicrophoneStatus = () => {
    if (settings?.always_on_microphone) return "active";
    return settings?.selected_microphone ? "idle" : "error";
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h1 className="text-2xl font-semibold text-gray-100">System Monitor</h1>
        <p className="text-sm text-gray-400 mt-1">Application logs and activity</p>
      </div>

      {/* Status Cards Grid */}
      <div className="px-6 py-4 border-b border-gray-800 bg-gray-900/30">
        <div className="grid grid-cols-4 gap-4">
          <StatusCard
            icon={Activity}
            title="Recording"
            value={isRecording ? "Active" : "Idle"}
            status={isRecording ? "active" : "idle"}
          />
          <StatusCard
            icon={Package}
            title="Model"
            value={modelStatus}
            status={getModelCardStatus()}
          />
          <StatusCard
            icon={Mic}
            title="Microphone"
            value={
              settings?.always_on_microphone
                ? "Always On"
                : settings?.selected_microphone || "None"
            }
            status={getMicrophoneStatus()}
          />
          <StatusCard
            icon={Shield}
            title="VAD"
            value="Silero (smoothed)"
            status="idle"
          />
        </div>
      </div>

      {/* Content - Two Panel Layout */}
      <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Logs List */}
          <div className="w-80 border-r border-gray-800 overflow-y-auto bg-gray-900/50">
            <LogsList
              selectedId={selectedLogId}
              onSelect={setSelectedLogId}
            />
          </div>

          {/* Right Panel - Log Details */}
          <div className="flex-1 overflow-y-auto">
            <LogsDetail logId={selectedLogId} />
          </div>
      </div>
    </div>
  );
}
