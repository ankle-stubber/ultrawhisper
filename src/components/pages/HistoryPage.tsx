import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { HistoryList } from "../history/HistoryList";
import { HistoryDetail } from "../history/HistoryDetail";
import { HistorySettings } from "../settings/HistorySettings";

export function HistoryPage() {
  const [selectedHistoryId, setSelectedHistoryId] = useState<string | null>("all-transcriptions");
  const [showSettings, setShowSettings] = useState(false);

  // Listen for history updates
  useEffect(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const unlisten = await listen("history-updated", () => {
        console.log("History updated event received");
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
            <h1 className="text-2xl font-semibold text-gray-100">History</h1>
            <p className="text-sm text-gray-400 mt-1">Browse past transcriptions</p>
          </div>
          <button
            onClick={() => setShowSettings(!showSettings)}
            className="px-3 py-1 text-xs bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-md transition-colors"
          >
            {showSettings ? "View History" : "Settings"}
          </button>
        </div>
      </div>

      {/* Content */}
      {showSettings ? (
        // History Settings View
        <div className="flex-1 overflow-y-auto p-6">
          <div className="max-w-2xl mx-auto">
            <HistorySettings />
          </div>
        </div>
      ) : (
        // History List/Detail View - Two Panel Layout
        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - History List */}
          <div className="w-80 border-r border-gray-800 overflow-y-auto bg-gray-900/50">
            <HistoryList
              selectedId={selectedHistoryId}
              onSelect={setSelectedHistoryId}
            />
          </div>

          {/* Right Panel - History Detail */}
          <div className="flex-1 overflow-y-auto">
            <HistoryDetail historyId={selectedHistoryId} />
          </div>
        </div>
      )}
    </div>
  );
}
