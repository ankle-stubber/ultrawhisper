import { z } from "zod";

export const ShortcutBindingSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  default_binding: z.string(),
  current_binding: z.string(),
  paste_to_window: z.boolean().optional().default(true),
  save_to_file: z.boolean().optional().default(false),
  output_path: z.string().nullable().optional(),
});

export const ShortcutBindingsMapSchema = z.record(
  z.string(),
  ShortcutBindingSchema,
);

export const AudioDeviceSchema = z.object({
  index: z.string(),
  name: z.string(),
  is_default: z.boolean(),
});

export const OverlayPositionSchema = z.enum(["none", "top", "bottom"]);
export type OverlayPosition = z.infer<typeof OverlayPositionSchema>;

export const ModelUnloadTimeoutSchema = z.enum([
  "never",
  "immediately",
  "min2",
  "min5",
  "min10",
  "min15",
  "hour1",
  "sec5",
]);
export type ModelUnloadTimeout = z.infer<typeof ModelUnloadTimeoutSchema>;

export const PasteMethodSchema = z.enum(["ctrl_v", "direct"]);
export type PasteMethod = z.infer<typeof PasteMethodSchema>;

export const ClipboardHandlingSchema = z.enum(["dont_modify", "copy_to_clipboard"]);
export type ClipboardHandling = z.infer<typeof ClipboardHandlingSchema>;

export const BatchTranscriptionSettingsSchema = z.object({
  enabled: z.boolean().optional().default(false),
  watch_folders: z.array(z.string()).optional().default([]),
  check_interval_seconds: z.number().optional().default(60),
  stability_timeout_seconds: z.number().optional().default(30),
  output_suffix: z.string().optional().default("_transcribed"),
  delete_after_transcription: z.boolean().optional().default(false),
  save_to_history: z.boolean().optional().default(false),
  min_file_size_kb: z.number().optional().default(1),
  max_file_size_mb: z.number().optional().default(500),
  output_folder: z.string().nullable().optional(),
  template_id: z.string().optional().default("default_markdown"),
  file_patterns: z.array(z.string()).optional().default(["*.wav"]),
});
export type BatchTranscriptionSettings = z.infer<typeof BatchTranscriptionSettingsSchema>;

export interface BatchCompleteEvent {
  processed: number;
  failed: number;
  timestamp: number;
}

export const BackpressurePolicySchema = z.enum(["Block", "DropNewest", "Coalesce"]);
export type BackpressurePolicy = z.infer<typeof BackpressurePolicySchema>;

export const StreamingSettingsSchema = z.object({
  enabled: z.boolean().optional().default(false),
  auto_enable_threshold_seconds: z.number().optional().default(300),
  chunk_duration_seconds: z.number().optional().default(20),
  overlap_seconds: z.number().optional().default(2),
  max_queue_size: z.number().optional().default(10),
  backpressure_policy: BackpressurePolicySchema.optional().default("Block"),
  save_streaming_audio: z.boolean().optional().default(true),
  enable_backfill: z.boolean().optional().default(false),
  writer_flush_interval_secs: z.number().optional().default(5),
  audio_format: z.string().optional().default("wav"),
});
export type StreamingSettings = z.infer<typeof StreamingSettingsSchema>;

export const CleaningRuleSchema = z.object({
  pattern: z.string(),
  replace: z.string(),
  flags: z.string().optional(),
});
export type CleaningRule = z.infer<typeof CleaningRuleSchema>;

export const CleaningSettingsSchema = z.object({
  enabled: z.boolean().optional().default(false),
  profile: z.string().optional().default("basic"),
  rules: z.array(CleaningRuleSchema).optional().default([]),
});
export type CleaningSettings = z.infer<typeof CleaningSettingsSchema>;

export const SettingsSchema = z.object({
  bindings: ShortcutBindingsMapSchema,
  push_to_talk: z.boolean(),
  audio_feedback: z.boolean(),
  audio_feedback_volume: z.number().optional().default(1.0),
  sound_theme: z
    .enum(["marimba", "pop", "custom"])
    .optional()
    .default("marimba"),
  start_hidden: z.boolean().optional().default(false),
  autostart_enabled: z.boolean().optional().default(false),
  selected_model: z.string(),
  always_on_microphone: z.boolean(),
  selected_microphone: z.string().nullable().optional(),
  selected_output_device: z.string().nullable().optional(),
  translate_to_english: z.boolean(),
  selected_language: z.string(),
  overlay_position: OverlayPositionSchema,
  debug_mode: z.boolean(),
  custom_words: z.array(z.string()).optional().default([]),
  model_unload_timeout: ModelUnloadTimeoutSchema.optional().default("never"),
  word_correction_threshold: z.number().optional().default(0.18),
  history_limit: z.number().optional().default(5),
  paste_method: PasteMethodSchema.optional().default("ctrl_v"),
  clipboard_handling: ClipboardHandlingSchema.optional().default("dont_modify"),
  batch_transcription: BatchTranscriptionSettingsSchema.optional().default({
    enabled: false,
    watch_folders: [],
    check_interval_seconds: 60,
    stability_timeout_seconds: 30,
    output_suffix: "_transcribed",
    delete_after_transcription: false,
    save_to_history: false,
    min_file_size_kb: 1,
    max_file_size_mb: 500,
  }),
  use_workflow_engine: z.boolean().optional().default(false),
  streaming: StreamingSettingsSchema.optional().default({
    enabled: false,
    auto_enable_threshold_seconds: 300,
    chunk_duration_seconds: 20,
    overlap_seconds: 2,
    max_queue_size: 10,
    backpressure_policy: "Block",
    save_streaming_audio: true,
    enable_backfill: true,
    writer_flush_interval_secs: 5,
    audio_format: "wav",
  }),
  cleaning: CleaningSettingsSchema.optional().default({
    enabled: false,
    profile: "basic",
    rules: [],
  }),
});

export const BindingResponseSchema = z.object({
  success: z.boolean(),
  binding: ShortcutBindingSchema.nullable(),
  error: z.string().nullable(),
});

export type AudioDevice = z.infer<typeof AudioDeviceSchema>;
export type BindingResponse = z.infer<typeof BindingResponseSchema>;
export type ShortcutBinding = z.infer<typeof ShortcutBindingSchema>;
export type ShortcutBindingsMap = z.infer<typeof ShortcutBindingsMapSchema>;
export type Settings = z.infer<typeof SettingsSchema>;

export const ModelInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  filename: z.string(),
  url: z.string().optional(),
  size_mb: z.number(),
  is_downloaded: z.boolean(),
  is_downloading: z.boolean(),
  partial_size: z.number(),
  is_directory: z.boolean(),
});

export type ModelInfo = z.infer<typeof ModelInfoSchema>;

// Log types for Bundle 6
export const LogEntrySchema = z.object({
  timestamp: z.number(),
  level: z.string(),
  target: z.string(),
  message: z.string(),
});

export type LogEntry = z.infer<typeof LogEntrySchema>;

export const LogFilterSchema = z.object({
  level: z.string().optional(),
  search: z.string().optional(),
  workflow: z.string().optional(),
});

export type LogFilter = z.infer<typeof LogFilterSchema>;

export type Category = "workflows" | "destinations" | "models" | "history" | "logs";

// Workflow types (Bundle 7)
export const TriggerConfigSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("Hotkey"),
    binding: z.string(),
    push_to_talk: z.boolean(),
  }),
  z.object({
    type: z.literal("FolderWatch"),
    paths: z.array(z.string()),
    file_patterns: z.array(z.string()),
    interval_seconds: z.number(),
    stability_timeout_seconds: z.number(),
  }),
]);

export const ModelConfigDtoSchema = z.object({
  model_id: z.string(),
  language: z.string(),
  translate_to_english: z.boolean(),
});

export const StoredWorkflowSchema = z.object({
  id: z.string(),
  name: z.string(),
  enabled: z.boolean(),
  trigger: TriggerConfigSchema,
  model: ModelConfigDtoSchema,
  destination_ids: z.array(z.string()),
  notes: z.string().optional(),
});

export type TriggerConfig = z.infer<typeof TriggerConfigSchema>;
export type ModelConfigDto = z.infer<typeof ModelConfigDtoSchema>;
export type StoredWorkflow = z.infer<typeof StoredWorkflowSchema>;

// Destination types (Bundle 7)
export const DestinationConfigSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("ActiveWindow"),
    paste_method: z.string().optional().default("ctrl_v"),
    preserve_clipboard: z.boolean().optional().default(false),
  }),
  z.object({
    type: z.literal("FileSystem"),
    path: z.string(),
    extension: z.string().optional().default("md"),
    filename_pattern: z.string().optional().default("transcription_{timestamp}.md"),
  }),
  z.object({
    type: z.literal("Telegram"),
    credential_id: z.string(),
    chat_id: z.string(),
    include_audio: z.boolean().optional().default(false),
  }),
]);

export const DestinationSchema = z.object({
  id: z.string(),
  name: z.string(),
  config: DestinationConfigSchema,
  template: z.string().optional(),
});

export type DestinationConfig = z.infer<typeof DestinationConfigSchema>;
export type Destination = z.infer<typeof DestinationSchema>;
