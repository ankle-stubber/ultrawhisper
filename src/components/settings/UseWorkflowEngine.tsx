import React from "react";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface UseWorkflowEngineProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const UseWorkflowEngine: React.FC<UseWorkflowEngineProps> = React.memo(({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const useWorkflowEngine = getSetting("use_workflow_engine") || false;

  return (
    <ToggleSwitch
      checked={useWorkflowEngine}
      onChange={(enabled) => updateSetting("use_workflow_engine", enabled)}
      isUpdating={isUpdating("use_workflow_engine")}
      label="Use Workflow Engine (experimental)"
      description="Route transcriptions through the new Workflow Engine. Falls back to legacy mode on error."
      descriptionMode={descriptionMode}
      grouped={grouped}
    />
  );
});
