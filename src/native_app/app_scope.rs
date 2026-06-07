//! Shared native-app names for the current flat module layout.
//!
//! Keep this file limited to names that are used across several native-app
//! modules. As feature areas move into focused submodules, prefer importing
//! from the owning module directly and shrinking this scope.

pub(super) use super::audio_settings::format_sample_rate_label;
pub(super) use super::file_actions::sample_path_label;
pub(super) use super::folder_browser::{
    FileMoveConflictResolution, FolderBrowserMessage, FolderBrowserState, FolderScanProgress,
};
pub(super) use super::launch::emit_gui_action;
pub(super) use super::layout::view;
pub(super) use super::sample_load_actions::{NormalizedWaveformReload, WaveformPlaybackResume};
pub(super) use super::shortcuts::default_gui_shortcut_resolution;
pub(super) use super::state::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, ActiveFolderCacheWarmResult,
    AppSettingsTab, AudioSettingsDropdown, DEFAULT_FOLDER_WIDTH, DRAG_PREVIEW_HEIGHT,
    DRAG_PREVIEW_MAX_WIDTH, FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID,
    FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, GENERAL_SETTINGS_BUTTON_HEIGHT,
    GENERAL_SETTINGS_BUTTON_ID, GENERAL_SETTINGS_BUTTON_WIDTH, GuiMessage,
    KEYBOARD_SAMPLE_LOAD_DEBOUNCE, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, NativeAppState,
    NativeFileDropHover, NormalizationProgress, NormalizationResult,
    PLAYBACK_START_ACTIVE_SOURCE_GRACE, PendingPlaybackStart, PendingSamplePlayback,
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS, SAMPLE_BROWSER_ROW_HEIGHT, SampleLoadResult,
    SampleNameViewMode, SamplePlaybackReady, TRANSACTION_LIST_MODAL_ID,
    UNCACHED_SAMPLE_LOAD_DEBOUNCE, VOLUME_PERSIST_DEBOUNCE, VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID,
    VOLUME_SLIDER_WIDTH, WAVEFORM_PANEL_HEIGHT, WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_VIEW_HEIGHT,
    WAVEFORM_WIDGET_ID, WaveformCacheEntry, WaveformCacheIndicatorRefreshResult,
    WaveformCacheWarmResult,
};
pub(super) use super::waveform::{
    WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState,
};
pub(super) use wavecrate::logging;
