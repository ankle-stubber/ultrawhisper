import React from "react";

interface PrimaryButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
  variant?: "primary" | "secondary" | "danger";
  size?: "sm" | "md" | "lg";
  fullWidth?: boolean;
}

export const PrimaryButton: React.FC<PrimaryButtonProps> = ({
  children,
  variant = "primary",
  size = "md",
  fullWidth = false,
  className = "",
  disabled,
  ...props
}) => {
  // Size classes
  const sizeClasses = {
    sm: "px-3 py-1 text-xs",
    md: "px-4 py-2 text-sm",
    lg: "px-6 py-3 text-base",
  };

  // Variant classes using theme tokens
  const variantClasses = {
    primary: disabled
      ? "uw-bg-primary-dim uw-text-secondary cursor-not-allowed opacity-50"
      : "uw-bg-primary text-gray-950 hover:opacity-90",
    secondary: disabled
      ? "uw-bg-primary-dim uw-text-secondary cursor-not-allowed opacity-50"
      : "uw-bg-primary-dim uw-text-accent border uw-border-primary hover:uw-bg-primary hover:text-gray-950",
    danger: disabled
      ? "uw-bg-error-dim uw-text-secondary cursor-not-allowed opacity-50"
      : "uw-bg-error-dim uw-text-error border uw-border-error hover:bg-red-500 hover:text-white",
  };

  const baseClasses =
    "rounded-lg font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-950";

  const widthClass = fullWidth ? "w-full" : "";

  return (
    <button
      className={`${baseClasses} ${sizeClasses[size]} ${variantClasses[variant]} ${widthClass} ${className}`}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
};
