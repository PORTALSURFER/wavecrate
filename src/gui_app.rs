//! Default Wavecrate GUI application built on Radiant's current public API.

#[cfg(test)]
pub(in crate::gui_app) use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, ResolvedOutput,
};
use wavecrate::logging;
#[cfg(test)]
pub(in crate::gui_app) use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

mod audio_engine;
mod audio_settings;
mod context_menu;
mod context_menu_actions;
mod drag_drop_actions;
mod file_actions;
mod folder_browser;
mod folder_browser_actions;
mod folder_browser_rename_actions;
mod folder_scan_actions;
mod launch;
mod layout;
mod lifecycle;
mod message_dispatch;
mod metadata_tag_metrics;
mod metadata_tags;
mod native_file_drop_actions;
mod normalization_actions;
mod playback;
mod sample_browser_view;
mod sample_collections;
mod sample_load_actions;
mod sample_ratings;
mod selected_file_actions;
mod shortcuts;
mod source_watcher;
mod state;
mod status_bar;
mod toolbar;
mod transaction_history;
mod trash_actions;
mod waveform;
mod waveform_panel;
#[cfg(test)]
use audio_settings::audio_settings_popover;
use audio_settings::format_sample_rate_label;
#[cfg(test)]
use audio_settings::top_status_bar;
#[cfg(test)]
pub(in crate::gui_app) use context_menu::{BrowserContextMenu, BrowserContextTargetKind};
#[cfg(test)]
use file_actions::format_copy_path;
#[cfg(test)]
use file_actions::normalize_wav_file_in_place;
use file_actions::sample_path_label;
use folder_browser::{
    FileMoveConflictResolution, FolderBrowserMessage, FolderBrowserState, FolderScanProgress,
};
use launch::emit_gui_action;
pub(crate) use launch::run;
#[cfg(test)]
use launch::{DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, debug_layout_requested};
use layout::view;
#[cfg(test)]
pub(in crate::gui_app) use metadata_tags::MetadataTagInputMode;
#[cfg(test)]
use sample_browser_view::sample_browser;
use sample_load_actions::{NormalizedWaveformReload, WaveformPlaybackResume};
use shortcuts::default_gui_shortcut_resolution;
#[cfg(test)]
pub(in crate::gui_app) use state::DEFAULT_VOLUME;
pub(in crate::gui_app) use state::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, ActiveFolderCacheWarmResult,
    AppSettingsTab, AudioSettingsDropdown, DEFAULT_FOLDER_WIDTH, DRAG_PREVIEW_HEIGHT,
    DRAG_PREVIEW_MAX_WIDTH, FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID,
    FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, GENERAL_SETTINGS_BUTTON_HEIGHT,
    GENERAL_SETTINGS_BUTTON_ID, GENERAL_SETTINGS_BUTTON_WIDTH, GuiAppState, GuiMessage,
    KEYBOARD_SAMPLE_LOAD_DEBOUNCE, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, NativeFileDropHover,
    NormalizationProgress, NormalizationResult, PLAYBACK_START_ACTIVE_SOURCE_GRACE,
    PendingPlaybackStart, PendingSamplePlayback, SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
    SAMPLE_BROWSER_ROW_HEIGHT, SampleLoadResult, SampleNameViewMode, TRANSACTION_LIST_MODAL_ID,
    UNCACHED_SAMPLE_LOAD_DEBOUNCE, VOLUME_PERSIST_DEBOUNCE, VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID,
    VOLUME_SLIDER_WIDTH, WAVEFORM_PANEL_HEIGHT, WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_VIEW_HEIGHT,
    WAVEFORM_WIDGET_ID, WaveformCacheEntry, WaveformCacheIndicatorRefreshResult,
    WaveformCacheWarmResult,
};
#[cfg(test)]
use toolbar::{
    TOOLBAR_FOCUS_LOADED_ID, TOOLBAR_RANDOM_ID, TOOLBAR_STOP_ID, ToolbarIcon, toolbar_icon_button,
    toolbar_icon_color, toolbar_icon_glyph,
};
use waveform::{WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState};

#[cfg(test)]
mod tests;
