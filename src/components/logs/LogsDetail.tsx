import React from "react";
import { LogViewer } from "./LogViewer";

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

  return (
    <div className="h-full">
      <LogViewer />
    </div>
  );
};
