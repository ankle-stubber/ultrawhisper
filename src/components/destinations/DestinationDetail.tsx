import React from "react";
import { TelegramSetup } from "./TelegramSetup";

interface DestinationDetailProps {
  destinationId: string | null;
}

export const DestinationDetail: React.FC<DestinationDetailProps> = ({
  destinationId,
}) => {
  if (!destinationId) {
    return (
      <div className="flex items-center justify-center h-full text-mid-gray">
        <p>Select a destination to configure</p>
      </div>
    );
  }

  // For MVP, only Telegram destination
  if (destinationId === "telegram-default") {
    return (
      <div className="flex-1 overflow-y-auto">
        <div className="flex flex-col items-center p-4 gap-4">
          <TelegramSetup credentialId="telegram_default" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-full text-mid-gray">
      <p>Destination not found</p>
    </div>
  );
};
