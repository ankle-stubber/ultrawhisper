import React from "react";

interface LogsDetailProps {
  logId: string | null;
}

export const LogsDetail: React.FC<LogsDetailProps> = ({ logId }) => {
  if (!logId) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Select logs to view</p>
      </div>
    );
  }

  // Placeholder - will be replaced with LogViewer in Phase 4
  return (
    <div className="flex items-center justify-center h-full text-mid-gray">
      <p>Log viewer coming in Phase 4...</p>
    </div>
  );
};
