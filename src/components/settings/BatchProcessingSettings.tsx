import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
// import { open } from "@tauri-apps/plugin-dialog";  // Temporarily disabled to debug
import { Trash2, Folder, Plus, Play } from "lucide-react";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import type { BatchTranscriptionSettings } from "../../lib/types";

const intervalOptions = [
  { value: 60, label: "1 minute" },
  { value: 300, label: "5 minutes" },
  { value: 900, label: "15 minutes" },
  { value: 1800, label: "30 minutes" },
  { value: 3600, label: "1 hour" },
];

export default function BatchProcessingSettings() {
  const { settings, refreshSettings } = useSettings();
  const [isProcessing, setIsProcessing] = useState(false);
  const [newFolder, setNewFolder] = useState("");

  const batchSettings = settings?.batch_transcription || {
    enabled: false,
    watch_folders: [],
    check_interval_seconds: 60,
    stability_timeout_seconds: 30,
    output_suffix: "_transcribed",
    delete_after_transcription: false,
    save_to_history: false,
    min_file_size_kb: 1,
    max_file_size_mb: 500,
  };

  const updateBatchSettings = useCallback(
    async (updates: Partial<BatchTranscriptionSettings>) => {
      const newSettings = { ...batchSettings, ...updates };

      try {
        await invoke("update_batch_settings", { batchSettings: newSettings });
        await refreshSettings();
        toast.success("Batch settings updated");
      } catch (error) {
        console.error("Failed to update batch settings:", error);
        toast.error("Failed to update batch settings");
      }
    },
    [batchSettings, refreshSettings]
  );

  const handleAddFolder = useCallback(async () => {
    // Temporarily using text input instead of folder picker
    if (!newFolder.trim()) {
      toast.error("Please enter a folder path");
      return;
    }

    if (batchSettings.watch_folders.includes(newFolder)) {
      toast.info("Folder already in watch list");
      return;
    }

    const updatedFolders = [...batchSettings.watch_folders, newFolder];
    await updateBatchSettings({ watch_folders: updatedFolders });
    setNewFolder("");
    toast.success("Folder added to watch list");
  }, [newFolder, batchSettings.watch_folders, updateBatchSettings]);

  const handleRemoveFolder = useCallback(
    async (folderPath: string) => {
      const updatedFolders = batchSettings.watch_folders.filter(
        (f) => f !== folderPath
      );
      await updateBatchSettings({ watch_folders: updatedFolders });
      toast.success("Folder removed from watch list");
    },
    [batchSettings.watch_folders, updateBatchSettings]
  );

  const handleProcessNow = useCallback(async () => {
    if (batchSettings.watch_folders.length === 0) {
      toast.error("No folders configured to watch");
      return;
    }

    setIsProcessing(true);
    try {
      const result = await invoke<{ processed: number; failed: number }>(
        "process_batch_now"
      );

      if (result.processed > 0) {
        toast.success(
          `Processed ${result.processed} file(s)${
            result.failed > 0 ? `, ${result.failed} failed` : ""
          }`
        );
      } else if (result.failed > 0) {
        toast.error(`Failed to process ${result.failed} file(s)`);
      } else {
        toast.info("No new files to process");
      }
    } catch (error) {
      console.error("Failed to process batch:", error);
      toast.error("Failed to process batch");
    } finally {
      setIsProcessing(false);
    }
  }, [batchSettings.watch_folders]);

  return (
    <div className="space-y-6">
      <div className="border rounded-lg p-4 space-y-4">
        <h3 className="text-lg font-semibold">Batch Transcription</h3>

        {/* Enable/Disable Toggle */}
        <div className="flex items-center justify-between">
          <div>
            <label htmlFor="batch-enabled" className="text-sm font-medium">
              Enable Batch Processing
            </label>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Automatically transcribe WAV files in watched folders
            </p>
          </div>
          <input
            id="batch-enabled"
            type="checkbox"
            className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            checked={batchSettings.enabled}
            onChange={(e) => updateBatchSettings({ enabled: e.target.checked })}
          />
        </div>

        {/* Watch Folders */}
        <div className="space-y-3">
          <label className="text-sm font-medium">Watch Folders</label>

          {/* Temporary text input for folder paths */}
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Enter folder path (e.g., /Users/greg/Documents/audio)"
              className="flex-1 px-3 py-2 border rounded-md dark:bg-gray-800 dark:border-gray-700"
              value={newFolder}
              onChange={(e) => setNewFolder(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddFolder()}
            />
            <button
              onClick={handleAddFolder}
              className="inline-flex items-center gap-1 px-3 py-2 text-sm rounded-md bg-blue-600 text-white hover:bg-blue-700 transition-colors"
              title="Add folder"
            >
              <Plus className="h-4 w-4" />
              Add
            </button>
          </div>

          {batchSettings.watch_folders.length === 0 ? (
            <p className="text-xs text-gray-500 dark:text-gray-400 italic">
              No folders configured. Enter a folder path above to add folders containing WAV files.
            </p>
          ) : (
            <div className="space-y-2">
              {batchSettings.watch_folders.map((folder) => (
                <div
                  key={folder}
                  className="flex items-center justify-between p-2 bg-gray-50 dark:bg-gray-800 rounded-md"
                >
                  <div className="flex items-center gap-2 text-sm truncate flex-1">
                    <Folder className="h-4 w-4 text-gray-500" />
                    <span className="truncate" title={folder}>
                      {folder}
                    </span>
                  </div>
                  <button
                    onClick={() => handleRemoveFolder(folder)}
                    className="p-1 text-red-500 hover:text-red-600 transition-colors"
                    title="Remove folder"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Check Interval */}
        <div>
          <label htmlFor="check-interval" className="text-sm font-medium">
            Check Interval
          </label>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 mb-2">
            How often to scan folders for new WAV files
          </p>
          <select
            id="check-interval"
            className="w-full px-3 py-2 border rounded-md dark:bg-gray-800 dark:border-gray-700"
            value={batchSettings.check_interval_seconds}
            onChange={(e) =>
              updateBatchSettings({
                check_interval_seconds: parseInt(e.target.value),
              })
            }
          >
            {intervalOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        {/* Advanced Settings */}
        <details className="border-t pt-4">
          <summary className="cursor-pointer text-sm font-medium mb-4">
            Advanced Settings
          </summary>

          <div className="space-y-4">
            {/* Save to History */}
            <div className="flex items-center justify-between">
              <div>
                <label htmlFor="save-to-history" className="text-sm font-medium">
                  Save to History
                </label>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Add batch transcriptions to app history (may flood history with large batches)
                </p>
              </div>
              <input
                id="save-to-history"
                type="checkbox"
                className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                checked={batchSettings.save_to_history}
                onChange={(e) =>
                  updateBatchSettings({ save_to_history: e.target.checked })
                }
              />
            </div>

            {/* File Stability Timeout */}
            <div>
              <label htmlFor="stability-timeout" className="text-sm font-medium">
                File Stability Timeout (seconds)
              </label>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 mb-2">
                Wait time after last modification before processing (default: 30s)
              </p>
              <input
                id="stability-timeout"
                type="number"
                min="1"
                max="300"
                className="w-full px-3 py-2 border rounded-md dark:bg-gray-800 dark:border-gray-700"
                value={batchSettings.stability_timeout_seconds}
                onChange={(e) =>
                  updateBatchSettings({
                    stability_timeout_seconds: parseInt(e.target.value) || 30,
                  })
                }
              />
            </div>

            {/* Output Suffix */}
            <div>
              <label htmlFor="output-suffix" className="text-sm font-medium">
                Output File Suffix
              </label>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 mb-2">
                Suffix added to transcribed files (e.g., file.wav → file_transcribed.md)
              </p>
              <input
                id="output-suffix"
                type="text"
                className="w-full px-3 py-2 border rounded-md dark:bg-gray-800 dark:border-gray-700"
                value={batchSettings.output_suffix}
                onChange={(e) =>
                  updateBatchSettings({ output_suffix: e.target.value })
                }
              />
            </div>
          </div>
        </details>

        {/* Process Now Button */}
        <div className="border-t pt-4">
          <button
            onClick={handleProcessNow}
            disabled={isProcessing || batchSettings.watch_folders.length === 0}
            className="inline-flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Play className="h-4 w-4" />
            {isProcessing ? "Processing..." : "Process Now"}
          </button>
          {batchSettings.enabled && (
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
              Automatic processing is {batchSettings.enabled ? "enabled" : "disabled"}.
              Next scan in ~{Math.floor(batchSettings.check_interval_seconds / 60)} minute(s).
            </p>
          )}
        </div>
      </div>

      {/* Instructions */}
      <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
        <h4 className="text-sm font-semibold text-blue-900 dark:text-blue-100 mb-2">
          How Batch Processing Works
        </h4>
        <ul className="text-xs text-blue-800 dark:text-blue-200 space-y-1 list-disc list-inside">
          <li>Add folders containing WAV audio files to monitor</li>
          <li>Files are automatically transcribed at the configured interval</li>
          <li>Transcriptions are saved as markdown files in the same folder</li>
          <li>Files must be stable (not modified) for {batchSettings.stability_timeout_seconds} seconds before processing</li>
          <li>Processed files are tracked to avoid re-transcription</li>
          <li>WAV files only in v1 (more formats coming soon)</li>
        </ul>
      </div>
    </div>
  );
}
