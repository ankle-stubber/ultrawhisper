import React from "react";

const UltraWhisperTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <div
      className={`font-bold ${className}`}
      style={{
        fontSize: width ? `${width / 8}px` : '24px',
        color: '#F9C5E8',
        fontFamily: 'system-ui, -apple-system, sans-serif'
      }}
    >
      UltraWhisper
    </div>
  );
};

export default UltraWhisperTextLogo;