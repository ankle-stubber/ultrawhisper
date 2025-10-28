import { Codesandbox } from "lucide-react";

const UltraWhisperIcon = ({
  width,
  height,
  size,
  className,
  ...props
}: {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}) => (
  <Codesandbox
    size={width || height || size || 24}
    className={className}
    {...props}
  />
);

export default UltraWhisperIcon;
