import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "compact";
}

export const Input: React.FC<InputProps> = ({
  className = "",
  variant = "default",
  ...props
}) => {
  const baseClasses = "px-2 py-1 text-sm font-semibold uw-bg-surface border uw-border-default rounded uw-text-primary text-left flex items-center justify-between transition-all duration-150 hover:uw-bg-card hover:uw-border-primary focus:outline-none focus:uw-bg-card focus:uw-border-primary";
  
  const variantClasses = {
    default: "px-3 py-2",
    compact: "px-2 py-1"
  };

  return (
    <input
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
      {...props}
    />
  );
};