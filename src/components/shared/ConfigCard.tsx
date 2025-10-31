import React from "react";

interface ConfigCardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export const ConfigCard: React.FC<ConfigCardProps> = ({
  title,
  children,
  className = "",
}) => {
  return (
    <div
      className={`uw-bg-elevated border uw-border-default rounded-xl p-6 mb-6 ${className}`}
    >
      {title && (
        <h3 className="text-lg font-semibold uw-text-primary mb-5">{title}</h3>
      )}
      <div className="space-y-5">{children}</div>
    </div>
  );
};

interface ConfigFieldProps {
  label: string;
  hint?: string;
  children: React.ReactNode;
}

export const ConfigField: React.FC<ConfigFieldProps> = ({
  label,
  hint,
  children,
}) => {
  return (
    <div className="space-y-2">
      <label className="block text-sm font-medium uw-text-primary">
        {label}
      </label>
      {children}
      {hint && <p className="text-xs uw-text-secondary">{hint}</p>}
    </div>
  );
};
