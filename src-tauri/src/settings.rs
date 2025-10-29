use crate::streaming::queue::BackpressurePolicy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
    #[serde(default = "default_paste_to_window")]
    pub paste_to_window: bool,
    #[serde(default = "default_save_to_file")]
    pub save_to_file: bool,
    #[serde(default)]
    pub output_path: Option<String>,
}

fn default_paste_to_window() -> bool {
    true
}

fn default_save_to_file() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OverlayPosition {
    None,
    Top,
    Bottom,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelUnloadTimeout {
    Never,
    Immediately,
    Min2,
    Min5,
    Min10,
    Min15,
    Hour1,
    Sec5, // Debug mode only
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    CtrlV,
    Direct,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardHandling {
    DontModify,
    CopyToClipboard,
}

impl Default for ModelUnloadTimeout {
    fn default() -> Self {
        ModelUnloadTimeout::Never
    }
}

impl Default for PasteMethod {
    fn default() -> Self {
        // Default to CtrlV for macOS and Windows, Direct for Linux
        #[cfg(target_os = "linux")]
        return PasteMethod::Direct;
        #[cfg(not(target_os = "linux"))]
        return PasteMethod::CtrlV;
    }
}

impl Default for ClipboardHandling {
    fn default() -> Self {
        ClipboardHandling::DontModify
    }
}

impl ModelUnloadTimeout {
    pub fn to_minutes(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Min2 => Some(2),
            ModelUnloadTimeout::Min5 => Some(5),
            ModelUnloadTimeout::Min10 => Some(10),
            ModelUnloadTimeout::Min15 => Some(15),
            ModelUnloadTimeout::Hour1 => Some(60),
            ModelUnloadTimeout::Sec5 => Some(0), // Special case for debug - handled separately
        }
    }

    pub fn to_seconds(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Sec5 => Some(5),
            _ => self.to_minutes().map(|m| m * 60),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundTheme {
    Marimba,
    Pop,
    Custom,
}

impl SoundTheme {
    fn as_str(&self) -> &'static str {
        match self {
            SoundTheme::Marimba => "marimba",
            SoundTheme::Pop => "pop",
            SoundTheme::Custom => "custom",
        }
    }

    pub fn to_start_path(&self) -> String {
        format!("resources/{}_start.wav", self.as_str())
    }

    pub fn to_stop_path(&self) -> String {
        format!("resources/{}_stop.wav", self.as_str())
    }
}

/// Batch transcription settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchTranscriptionSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub watch_folders: Vec<String>,
    #[serde(default = "default_check_interval_seconds")]
    pub check_interval_seconds: u64,
    #[serde(default = "default_stability_timeout_seconds")]
    pub stability_timeout_seconds: u64,
    #[serde(default = "default_output_suffix")]
    pub output_suffix: String,
    #[serde(default)]
    pub delete_after_transcription: bool,
    #[serde(default)]
    pub save_to_history: bool,
    #[serde(default = "default_min_file_size_kb")]
    pub min_file_size_kb: u64,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    /// Optional output folder for transcriptions. If None, saves to source folder
    #[serde(default)]
    pub output_folder: Option<String>,
    /// Template ID for output formatting
    #[serde(default = "default_template_id")]
    pub template_id: String,
}

impl Default for BatchTranscriptionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            watch_folders: Vec::new(),
            check_interval_seconds: default_check_interval_seconds(),
            stability_timeout_seconds: default_stability_timeout_seconds(),
            output_suffix: default_output_suffix(),
            delete_after_transcription: false,
            save_to_history: false,
            min_file_size_kb: default_min_file_size_kb(),
            max_file_size_mb: default_max_file_size_mb(),
            output_folder: None,
            template_id: default_template_id(),
        }
    }
}

fn default_check_interval_seconds() -> u64 {
    60 // 1 minute
}

fn default_stability_timeout_seconds() -> u64 {
    30 // 30 seconds
}

fn default_output_suffix() -> String {
    "_transcribed".to_string()
}

fn default_min_file_size_kb() -> u64 {
    1 // 1 KB minimum
}

fn default_max_file_size_mb() -> u64 {
    500 // 500 MB maximum
}

fn default_template_id() -> String {
    "default_markdown".to_string()
}

/// Streaming transcription settings for Phase 2
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamingSettings {
    /// Master switch - enable/disable streaming globally
    #[serde(default)]
    pub enabled: bool,

    /// Auto-enable streaming for recordings longer than N seconds (0 = disabled)
    #[serde(default = "default_auto_enable_threshold_seconds")]
    pub auto_enable_threshold_seconds: u32,

    /// Duration of each chunk in seconds
    #[serde(default = "default_chunk_duration_seconds")]
    pub chunk_duration_seconds: u32,

    /// Overlap between chunks in seconds
    #[serde(default = "default_overlap_seconds")]
    pub overlap_seconds: u32,

    /// Maximum number of chunks that can be queued
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,

    /// Policy for handling backpressure when queue is full
    #[serde(default)]
    pub backpressure_policy: BackpressurePolicy,

    /// Phase 3: Save streaming audio to disk
    #[serde(default = "default_save_streaming_audio")]
    pub save_streaming_audio: bool,

    /// Phase 3: Enable whole-file backfill after recording
    #[serde(default = "default_enable_backfill")]
    pub enable_backfill: bool,

    /// Phase 3: Flush interval for audio writer in seconds
    #[serde(default = "default_writer_flush_interval_secs")]
    pub writer_flush_interval_secs: u32,

    /// Phase 3: Audio format (currently only "wav" supported)
    #[serde(default = "default_audio_format")]
    pub audio_format: String,
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_enable_threshold_seconds: default_auto_enable_threshold_seconds(),
            chunk_duration_seconds: default_chunk_duration_seconds(),
            overlap_seconds: default_overlap_seconds(),
            max_queue_size: default_max_queue_size(),
            backpressure_policy: BackpressurePolicy::Block,
            save_streaming_audio: default_save_streaming_audio(),
            enable_backfill: default_enable_backfill(),
            writer_flush_interval_secs: default_writer_flush_interval_secs(),
            audio_format: default_audio_format(),
        }
    }
}

fn default_auto_enable_threshold_seconds() -> u32 {
    300 // 5 minutes - disabled by default when streaming is off
}

fn default_chunk_duration_seconds() -> u32 {
    20 // 20 seconds per chunk
}

fn default_overlap_seconds() -> u32 {
    2 // 2 seconds overlap
}

fn default_max_queue_size() -> usize { 10 }

fn default_save_streaming_audio() -> bool {
    true
}

fn default_enable_backfill() -> bool {
    true
}

fn default_writer_flush_interval_secs() -> u32 {
    5
}

fn default_audio_format() -> String {
    "wav".to_string()
}

/* still handy for composing the initial JSON in the store ------------- */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub bindings: HashMap<String, ShortcutBinding>,
    pub push_to_talk: bool,
    pub audio_feedback: bool,
    #[serde(default = "default_audio_feedback_volume")]
    pub audio_feedback_volume: f32,
    #[serde(default = "default_sound_theme")]
    pub sound_theme: SoundTheme,
    #[serde(default = "default_start_hidden")]
    pub start_hidden: bool,
    #[serde(default = "default_autostart_enabled")]
    pub autostart_enabled: bool,
    #[serde(default = "default_model")]
    pub selected_model: String,
    #[serde(default = "default_always_on_microphone")]
    pub always_on_microphone: bool,
    #[serde(default)]
    pub selected_microphone: Option<String>,
    #[serde(default)]
    pub selected_output_device: Option<String>,
    #[serde(default = "default_translate_to_english")]
    pub translate_to_english: bool,
    #[serde(default = "default_selected_language")]
    pub selected_language: String,
    #[serde(default = "default_overlay_position")]
    pub overlay_position: OverlayPosition,
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default)]
    pub custom_words: Vec<String>,
    #[serde(default)]
    pub model_unload_timeout: ModelUnloadTimeout,
    #[serde(default = "default_word_correction_threshold")]
    pub word_correction_threshold: f64,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default)]
    pub paste_method: PasteMethod,
    #[serde(default)]
    pub clipboard_handling: ClipboardHandling,
    #[serde(default)]
    pub batch_transcription: BatchTranscriptionSettings,
    #[serde(default)]
    pub use_workflow_engine: bool,
    #[serde(default)]
    pub streaming: StreamingSettings,
}

fn default_model() -> String {
    "".to_string()
}

fn default_always_on_microphone() -> bool {
    false
}

fn default_translate_to_english() -> bool {
    false
}

fn default_start_hidden() -> bool {
    false
}

fn default_autostart_enabled() -> bool {
    false
}

fn default_selected_language() -> String {
    "auto".to_string()
}

fn default_overlay_position() -> OverlayPosition {
    #[cfg(target_os = "linux")]
    return OverlayPosition::None;
    #[cfg(not(target_os = "linux"))]
    return OverlayPosition::Bottom;
}

fn default_debug_mode() -> bool {
    false
}

fn default_word_correction_threshold() -> f64 {
    0.18
}

fn default_history_limit() -> usize {
    5
}

fn default_audio_feedback_volume() -> f32 {
    1.0
}

fn default_sound_theme() -> SoundTheme {
    SoundTheme::Marimba
}

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

pub fn get_default_settings() -> AppSettings {
    #[cfg(target_os = "windows")]
    let default_shortcut = "ctrl+space";
    #[cfg(target_os = "macos")]
    let default_shortcut = "option+space";
    #[cfg(target_os = "linux")]
    let default_shortcut = "ctrl+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_shortcut = "alt+space";

    // Define the second shortcut for file output
    #[cfg(target_os = "windows")]
    let file_shortcut = "ctrl+alt+t";
    #[cfg(target_os = "macos")]
    let file_shortcut = "cmd+option+t";
    #[cfg(target_os = "linux")]
    let file_shortcut = "ctrl+alt+t";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let file_shortcut = "ctrl+alt+t";

    let mut bindings = HashMap::new();
    bindings.insert(
        "transcribe".to_string(),
        ShortcutBinding {
            id: "transcribe".to_string(),
            name: "Transcribe".to_string(),
            description: "Converts your speech into text.".to_string(),
            default_binding: default_shortcut.to_string(),
            current_binding: default_shortcut.to_string(),
            paste_to_window: true,
            save_to_file: false,
            output_path: None,
        },
    );
    bindings.insert(
        "transcribe_to_file".to_string(),
        ShortcutBinding {
            id: "transcribe_to_file".to_string(),
            name: "Save to File".to_string(),
            description: "Saves transcription as markdown file.".to_string(),
            default_binding: file_shortcut.to_string(),
            current_binding: file_shortcut.to_string(),
            paste_to_window: false,
            save_to_file: true,
            output_path: Some("Documents/UltraWhisper".to_string()),
        },
    );

    AppSettings {
        bindings,
        push_to_talk: true,
        audio_feedback: false,
        audio_feedback_volume: default_audio_feedback_volume(),
        sound_theme: default_sound_theme(),
        start_hidden: default_start_hidden(),
        autostart_enabled: default_autostart_enabled(),
        selected_model: "".to_string(),
        always_on_microphone: false,
        selected_microphone: None,
        selected_output_device: None,
        translate_to_english: false,
        selected_language: "auto".to_string(),
        overlay_position: OverlayPosition::Bottom,
        debug_mode: false,
        custom_words: Vec::new(),
        model_unload_timeout: ModelUnloadTimeout::Never,
        word_correction_threshold: default_word_correction_threshold(),
        history_limit: default_history_limit(),
        paste_method: PasteMethod::default(),
        clipboard_handling: ClipboardHandling::default(),
        batch_transcription: BatchTranscriptionSettings::default(),
        use_workflow_engine: false,
        streaming: StreamingSettings::default(),
    }
}

pub fn load_or_create_app_settings(app: &AppHandle) -> AppSettings {
    // Initialize store
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    let mut settings = if let Some(settings_value) = store.get("settings") {
        // Parse the entire settings object
        match serde_json::from_value::<AppSettings>(settings_value) {
            Ok(settings) => {
                println!("Found existing settings: {:?}", settings);

                settings
            }
            Err(e) => {
                println!("Failed to parse settings: {}", e);
                // Fall back to default settings if parsing fails
                let default_settings = get_default_settings();

                // Store the default settings
                store.set("settings", serde_json::to_value(&default_settings).unwrap());

                default_settings
            }
        }
    } else {
        // Create default settings
        let default_settings = get_default_settings();

        // Store the settings
        store.set("settings", serde_json::to_value(&default_settings).unwrap());

        default_settings
    };

    // Migration: ensure the file-output binding exists for existing users
    if !settings.bindings.contains_key("transcribe_to_file") {
        #[cfg(target_os = "windows")]
        let file_shortcut = "ctrl+alt+t";
        #[cfg(target_os = "macos")]
        let file_shortcut = "cmd+option+t";
        #[cfg(target_os = "linux")]
        let file_shortcut = "ctrl+alt+t";
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let file_shortcut = "ctrl+alt+t";

        settings.bindings.insert(
            "transcribe_to_file".to_string(),
            ShortcutBinding {
                id: "transcribe_to_file".to_string(),
                name: "Save to File".to_string(),
                description: "Saves transcription as markdown file.".to_string(),
                default_binding: file_shortcut.to_string(),
                current_binding: file_shortcut.to_string(),
                paste_to_window: false,
                save_to_file: true,
                output_path: Some("Documents/UltraWhisper".to_string()),
            },
        );

        // Persist the migrated settings
        store.set("settings", serde_json::to_value(&settings).unwrap());
    }

    settings
}

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    if let Some(settings_value) = store.get("settings") {
        serde_json::from_value::<AppSettings>(settings_value)
            .unwrap_or_else(|_| get_default_settings())
    } else {
        get_default_settings()
    }
}

pub fn write_settings(app: &AppHandle, settings: AppSettings) {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    store.set("settings", serde_json::to_value(&settings).unwrap());
}

pub fn get_bindings(app: &AppHandle) -> HashMap<String, ShortcutBinding> {
    let settings = get_settings(app);

    settings.bindings
}

pub fn get_stored_binding(app: &AppHandle, id: &str) -> ShortcutBinding {
    let bindings = get_bindings(app);

    let binding = bindings.get(id).unwrap().clone();

    binding
}

pub fn get_history_limit(app: &AppHandle) -> usize {
    let settings = get_settings(app);
    settings.history_limit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_settings_defaults() {
        let settings = StreamingSettings::default();

        // Verify Phase 2 defaults
        assert_eq!(settings.enabled, false);
        assert_eq!(settings.auto_enable_threshold_seconds, 300);
        assert_eq!(settings.chunk_duration_seconds, 20);
        assert_eq!(settings.overlap_seconds, 2);
        assert_eq!(settings.max_queue_size, 10);
        assert_eq!(settings.backpressure_policy, BackpressurePolicy::Block);

        // Verify Phase 3 defaults
        assert_eq!(settings.save_streaming_audio, true);
        assert_eq!(settings.enable_backfill, true);
        assert_eq!(settings.writer_flush_interval_secs, 5);
        assert_eq!(settings.audio_format, "wav");
    }

    #[test]
    fn test_streaming_settings_serde_roundtrip() {
        let settings = StreamingSettings::default();

        // Serialize and deserialize
        let json = serde_json::to_string(&settings).expect("Failed to serialize");
        let deserialized: StreamingSettings = serde_json::from_str(&json)
            .expect("Failed to deserialize");

        // Verify all fields match
        assert_eq!(deserialized.enabled, settings.enabled);
        assert_eq!(deserialized.save_streaming_audio, settings.save_streaming_audio);
        assert_eq!(deserialized.enable_backfill, settings.enable_backfill);
        assert_eq!(deserialized.writer_flush_interval_secs, settings.writer_flush_interval_secs);
        assert_eq!(deserialized.audio_format, settings.audio_format);
    }

    #[test]
    fn test_streaming_settings_default_functions() {
        // Verify individual default functions
        assert_eq!(default_save_streaming_audio(), true);
        assert_eq!(default_enable_backfill(), true);
        assert_eq!(default_writer_flush_interval_secs(), 5);
        assert_eq!(default_audio_format(), "wav");
    }
}
