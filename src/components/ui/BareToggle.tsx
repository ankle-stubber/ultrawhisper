import React from "react";

interface BareToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export const BareToggle: React.FC<BareToggleProps> = ({
  checked,
  onChange,
  disabled = false,
}) => {
  return (
    <label
      className={`inline-flex items-center ${disabled ? "cursor-not-allowed" : "cursor-pointer"}`}
    >
      <input
        type="checkbox"
        value=""
        className="sr-only peer"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <div className="relative w-11 h-6 uw-bg-card peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-[var(--uw-primary)] rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:uw-bg-primary peer-disabled:opacity-50"></div>
    </label>
  );
};
