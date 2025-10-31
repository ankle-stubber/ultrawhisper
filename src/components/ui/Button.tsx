import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "ghost";
  size?: "sm" | "md" | "lg";
}

export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  const baseClasses =
    "font-medium rounded focus:outline-none transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

  const variantClasses = {
    primary:
      "text-gray-950 uw-bg-primary hover:opacity-90 focus:ring-1 focus:uw-border-primary",
    secondary: "uw-bg-primary-dim uw-text-accent border uw-border-primary hover:uw-bg-primary hover:text-gray-950 focus:outline-none",
    danger:
      "uw-bg-error-dim uw-text-error hover:uw-bg-error hover:text-gray-950 focus:ring-1 focus:uw-border-error",
    ghost: "uw-text-primary hover:uw-bg-card focus:uw-bg-card",
  };

  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-4 py-[5px] text-sm",
    lg: "px-4 py-2 text-base",
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
