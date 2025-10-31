import React from "react";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface StreamingSettingsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const StreamingSettings: React.FC<StreamingSettingsProps> = React.memo(({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const streamingSettings = getSetting("streaming") || {
    enabled: false,
    auto_enable_threshold_seconds: 300,
    chunk_duration_seconds: 20,
    overlap_seconds: 2,
    max_queue_size: 10,
    backpressure_policy: "Block",
    save_streaming_audio: true,
    enable_backfill: false,
    writer_flush_interval_secs: 5,
    audio_format: "wav",
  };

  const updateStreamingSetting = <K extends keyof typeof streamingSettings>(
    key: K,
    value: typeof streamingSettings[K]
  ) => {
    updateSetting("streaming", {
      ...streamingSettings,
      [key]: value,
    });
  };

  return (
    <div className="space-y-4">
      <ToggleSwitch
        checked={streamingSettings.enabled}
        onChange={(enabled) => updateStreamingSetting("enabled", enabled)}
        isUpdating={isUpdating("streaming")}
        label="Enable Streaming Mode"
        description="Process long recordings with chunked streaming transcription instead of batch mode. Reduces memory usage for recordings longer than 20 seconds."
        descriptionMode={descriptionMode}
        grouped={grouped}
      />

      <p className="text-xs text-gray-500 dark:text-gray-400">
        Note: Streaming uses the Workflow Engine under the hood, even if the
        “Use Workflow Engine” toggle is off.
      </p>

      {streamingSettings.enabled && (
        <div className="ml-6 space-y-3 pt-2 border-l-2 border-gray-200 dark:border-gray-700 pl-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Chunk Duration (seconds)
            </label>
            <input
              type="number"
              min={5}
              max={60}
              value={streamingSettings.chunk_duration_seconds}
              onChange={(e) =>
                updateStreamingSetting("chunk_duration_seconds", parseInt(e.target.value))
              }
              className="w-32 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Duration of each audio chunk sent for transcription (default: 20)
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Overlap (seconds)
            </label>
            <input
              type="number"
              min={0}
              max={10}
              value={streamingSettings.overlap_seconds}
              onChange={(e) =>
                updateStreamingSetting("overlap_seconds", parseInt(e.target.value))
              }
              className="w-32 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Overlap between chunks for seamless merging (default: 2)
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Max Queue Size
            </label>
            <input
              type="number"
              min={1}
              max={20}
              value={streamingSettings.max_queue_size}
              onChange={(e) =>
                updateStreamingSetting("max_queue_size", parseInt(e.target.value))
              }
              className="w-32 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Maximum chunks in processing queue (default: 10)
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Backpressure Policy
            </label>
            <select
              value={streamingSettings.backpressure_policy}
              onChange={(e) =>
                updateStreamingSetting("backpressure_policy", e.target.value as any)
              }
              className="w-48 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            >
              <option value="DropNewest">Drop Newest</option>
              <option value="Block">Block (May cause audio dropouts)</option>
              <option value="Coalesce">Coalesce (Future)</option>
            </select>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              How to handle full queue: drop newest chunk or block (default: Block)
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Auto-Enable Threshold (seconds, 0=disabled)
            </label>
            <input
              type="number"
              min={0}
              max={3600}
              step={30}
              value={streamingSettings.auto_enable_threshold_seconds}
              onChange={(e) =>
                updateStreamingSetting("auto_enable_threshold_seconds", parseInt(e.target.value))
              }
              className="w-32 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Future: Auto-enable streaming for recordings longer than N seconds (default: 300)
            </p>
          </div>

          <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
              Audio Storage & Backfill
            </h3>

            <div className="space-y-3">
              <ToggleSwitch
                checked={streamingSettings.save_streaming_audio}
                onChange={(enabled) => updateStreamingSetting("save_streaming_audio", enabled)}
                isUpdating={isUpdating("streaming")}
                label="Save Streaming Audio"
                description="Write 16kHz mono WAV file during streaming for history playback"
                descriptionMode={descriptionMode}
                grouped={grouped}
              />

              <ToggleSwitch
                checked={streamingSettings.enable_backfill}
                onChange={(enabled) => updateStreamingSetting("enable_backfill", enabled)}
                isUpdating={isUpdating("streaming")}
                label="Enable Backfill"
                description="After recording, re-transcribe the saved WAV for improved accuracy"
                descriptionMode={descriptionMode}
                grouped={grouped}
              />

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Writer Flush Interval (seconds)
                </label>
                <input
                  type="number"
                  min={1}
                  max={30}
                  value={streamingSettings.writer_flush_interval_secs}
                  onChange={(e) =>
                    updateStreamingSetting("writer_flush_interval_secs", parseInt(e.target.value))
                  }
                  className="w-32 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                />
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  How often to flush audio data to disk (default: 5)
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Audio Format
                </label>
                <select
                  value={streamingSettings.audio_format}
                  onChange={(e) =>
                    updateStreamingSetting("audio_format", e.target.value)
                  }
                  className="w-48 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                >
                  <option value="wav">WAV (16-bit PCM)</option>
                  <option value="opus" disabled>OPUS (Coming Soon)</option>
                  <option value="flac" disabled>FLAC (Coming Soon)</option>
                </select>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Format for saved audio files (currently only WAV supported)
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
