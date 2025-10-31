import React from "react";

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
}

export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  subtitle,
  actions,
}) => {
  return (
    <div className="px-6 py-4 border-b uw-border-default">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold uw-text-primary">{title}</h1>
          {subtitle && (
            <p className="text-sm uw-text-secondary mt-1">{subtitle}</p>
          )}
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </div>
    </div>
  );
};
