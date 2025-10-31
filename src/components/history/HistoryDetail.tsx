import React from "react";
import { HistorySettings } from "../settings/HistorySettings";

interface HistoryDetailProps {
  historyId: string | null;
}

export const HistoryDetail: React.FC<HistoryDetailProps> = ({ historyId }) => {
  if (!historyId) {
    return (
      <div className="flex items-center justify-center h-full uw-text-secondary">
        <p>Select history view</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto uw-scroll">
      <div className="flex flex-col items-center p-4 gap-4">
        <HistorySettings />
      </div>
    </div>
  );
};
