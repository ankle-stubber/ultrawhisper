import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { LogsList } from "../logs/LogsList";
import { LogsDetail } from "../logs/LogsDetail";

export function SystemMonitorPage() {
  const [selectedLogId, setSelectedLogId] = useState<string | null>("application-logs");
  const [isRecording, setIsRecording] = useState(false);
  const [modelStatus, setModelStatus] = useState<string>("Ready");

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

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold text-gray-100">System Monitor</h1>
            <p className="text-sm text-gray-400 mt-1">Application logs and activity</p>
          </div>

          {/* Optional status chips for Session 1 */}
          <div className="flex items-center gap-2">
            <div className={`px-3 py-1 rounded-full text-xs font-medium ${
              modelStatus === "Ready"
                ? "bg-green-500/10 text-green-500 border border-green-500/20"
                : "bg-gray-800 text-gray-400 border border-gray-700"
            }`}>
              Model: {modelStatus}
            </div>
            {isRecording && (
              <div className="px-3 py-1 rounded-full text-xs font-medium bg-red-500/10 text-red-500 border border-red-500/20 animate-pulse">
                Recording
              </div>
            )}
          </div>
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
