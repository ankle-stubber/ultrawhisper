import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "../../hooks/useSettings";
import { ShortcutBinding } from "../../lib/types";
import { Folder, X } from "lucide-react";
import { toast } from "sonner";
import { homeDir } from "@tauri-apps/api/path";

interface ShortcutOutputConfigProps {
  binding: ShortcutBinding;
}

export const ShortcutOutputConfig: React.FC<ShortcutOutputConfigProps> = ({
  binding,
}) => {
  const { updateBindingOutputConfig, isUpdating } = useSettings();

  // Guard against null/undefined binding
  if (!binding) {
    return null;
  }
  const [outputPath, setOutputPath] = useState(binding.output_path || "");
  const [pasteToWindow, setPasteToWindow] = useState(
    binding.paste_to_window ?? true
  );
  const [saveToFile, setSaveToFile] = useState(binding.save_to_file ?? false);
  const [homeDirPath, setHomeDirPath] = useState<string>("");

  // Sync local state with binding prop when it changes
  useEffect(() => {
    setOutputPath(binding.output_path || "");
    setPasteToWindow(binding.paste_to_window ?? true);
    setSaveToFile(binding.save_to_file ?? false);
  }, [binding.output_path, binding.paste_to_window, binding.save_to_file]);

  // Get the home directory path on component mount
  useEffect(() => {
    const getHomeDir = async () => {
      try {
        const home = await homeDir();
        setHomeDirPath(home);
      } catch (error) {
        console.error("Failed to get home directory:", error);
      }
    };
    getHomeDir();
  }, []);

  const handleBrowse = async () => {
    try {
      const selectedPath = await invoke<string | null>("pick_directory");
      if (selectedPath) {
        setOutputPath(selectedPath);
        await handleSave(pasteToWindow, saveToFile, selectedPath);
      }
    } catch (error) {
      console.error("Directory picker error:", error);
      toast.error(`Failed to pick directory: ${error}`);
    }
  };

  const handleSave = async (
    newPasteToWindow: boolean,
    newSaveToFile: boolean,
    newOutputPath: string
  ) => {
    try {
      await updateBindingOutputConfig(
        binding.id,
        newPasteToWindow,
        newSaveToFile,
        newOutputPath || null
      );
    } catch (error) {
      console.error("Failed to update binding config:", error);
      toast.error(`Failed to update output configuration: ${error}`);
    }
  };

  const handlePasteToggle = async (checked: boolean) => {
    setPasteToWindow(checked);
    await handleSave(checked, saveToFile, outputPath);
  };

  const handleSaveToggle = async (checked: boolean) => {
    setSaveToFile(checked);
    await handleSave(pasteToWindow, checked, outputPath);
  };

  const handleClearPath = async () => {
    setOutputPath("");
    await handleSave(pasteToWindow, saveToFile, "");
  };

  const formatPath = (path: string): string => {
    // Replace home directory with ~ for display
    if (!path) return "";

    // Use the home directory path we fetched
    if (homeDirPath && path.startsWith(homeDirPath)) {
      return "~" + path.slice(homeDirPath.length);
    }

    // Also handle paths that already start with ~
    if (path.startsWith("~/")) {
      return path;
    }

    return path;
  };

  return (
    <div className="space-y-3 border-t border-mid-gray/20 pt-3 mt-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-mid-gray">Output Options</span>
      </div>

      {/* Paste to Window Toggle */}
      <div className="flex items-center justify-between">
        <label className="text-xs text-mid-gray">Paste to active window</label>
        <input
          type="checkbox"
          checked={pasteToWindow}
          onChange={(e) => handlePasteToggle(e.target.checked)}
          disabled={isUpdating(`binding_output_${binding.id}`)}
          className="w-4 h-4 text-logo-primary rounded"
        />
      </div>

      {/* Save to File Toggle */}
      <div className="flex items-center justify-between">
        <label className="text-xs text-mid-gray">Save to file</label>
        <input
          type="checkbox"
          checked={saveToFile}
          onChange={(e) => handleSaveToggle(e.target.checked)}
          disabled={isUpdating(`binding_output_${binding.id}`)}
          className="w-4 h-4 text-logo-primary rounded"
        />
      </div>

      {/* File Path Configuration */}
      {saveToFile && (
        <div className="space-y-2">
          <label className="text-xs text-mid-gray">Output directory</label>
          <div className="flex items-center gap-1">
            <input
              type="text"
              className="flex-1 px-2 py-1 text-xs bg-mid-gray/10 rounded border border-mid-gray/20 font-mono"
              placeholder="Default: ~/Documents/UltraWhisper"
              value={formatPath(outputPath)}
              onChange={(e) => setOutputPath(e.target.value)}
              onBlur={() => handleSave(pasteToWindow, saveToFile, outputPath)}
              disabled={isUpdating(`binding_output_${binding.id}`)}
              title="Enter folder path or use browse button"
            />
            <button
              onClick={handleBrowse}
              disabled={isUpdating(`binding_output_${binding.id}`)}
              className="p-1 hover:bg-logo-primary/10 rounded"
              title="Browse for folder (Note: Currently causes app to freeze)"
            >
              <Folder className="w-4 h-4" />
            </button>
            {outputPath && (
              <button
                onClick={handleClearPath}
                disabled={isUpdating(`binding_output_${binding.id}`)}
                className="p-1 hover:bg-red-500/10 rounded"
                title="Clear custom path"
              >
                <X className="w-4 h-4 text-red-500" />
              </button>
            )}
          </div>
          <p className="text-xs text-mid-gray/60 mt-1">
            Type a path manually or leave blank for default. The browse button currently has issues.
          </p>
        </div>
      )}

      {/* Status Indicator */}
      <div className="text-xs text-mid-gray/60">
        {pasteToWindow && saveToFile
          ? "Will paste and save to file"
          : pasteToWindow
          ? "Will paste to active window"
          : saveToFile
          ? "Will save to file only"
          : "No output configured"}
      </div>
    </div>
  );
};