import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <div className="space-y-2 w-full">
      {title && (
        <div className="px-4">
          <h2 className="text-xs font-medium uw-text-secondary uppercase tracking-wide">
            {title}
          </h2>
          {description && (
            <p className="text-xs uw-text-secondary mt-1">{description}</p>
          )}
        </div>
      )}
      <div className="uw-bg-elevated border uw-border-default rounded-lg overflow-visible">
        <div className="divide-y uw-border-subtle">{children}</div>
      </div>
    </div>
  );
};
