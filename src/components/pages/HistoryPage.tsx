import { useState, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { HistoryList } from "../history/HistoryList";
import { HistoryDetail } from "../history/HistoryDetail";
import { HistorySettings } from "../settings/HistorySettings";
import { PageHeader, PrimaryButton } from "../shared";

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
    <div className="flex-1 flex flex-col h-full uw-bg-surface">
      {/* Header */}
      <PageHeader
        title="History"
        subtitle="Browse past transcriptions"
        actions={
          <PrimaryButton
            onClick={() => setShowSettings(!showSettings)}
            variant="secondary"
            size="sm"
          >
            {showSettings ? "View History" : "Settings"}
          </PrimaryButton>
        }
      />

      {/* Content */}
      {showSettings ? (
        // History Settings View
        <div className="flex-1 overflow-y-auto uw-scroll p-6">
          <div className="max-w-2xl mx-auto">
            <HistorySettings />
          </div>
        </div>
      ) : (
        // History List/Detail View - Two Panel Layout
        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - History List */}
          <div className="w-80 border-r uw-border-default overflow-y-auto uw-scroll uw-bg-elevated">
            <HistoryList
              selectedId={selectedHistoryId}
              onSelect={setSelectedHistoryId}
            />
          </div>

          {/* Right Panel - History Detail */}
          <div className="flex-1 overflow-y-auto uw-scroll">
            <HistoryDetail historyId={selectedHistoryId} />
          </div>
        </div>
      )}
    </div>
  );
}
