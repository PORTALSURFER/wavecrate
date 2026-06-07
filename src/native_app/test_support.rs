//! Test-only native-app accessors.

pub(in crate::native_app) use super::library_browser::context_menu::{
    BrowserContextMenu, BrowserContextTargetKind,
};
pub(in crate::native_app) use super::metadata::MetadataTagInputMode;
pub(in crate::native_app) use crate::native_app::app::*;
pub(in crate::native_app) use crate::native_app::app_chrome::library_browser::sample_browser_view::SampleFileHitTarget;
pub(in crate::native_app) use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
pub(in crate::native_app) use crate::native_app::app_chrome::toolbar::{
    TOOLBAR_FOCUS_LOADED_ID, TOOLBAR_RANDOM_ID, TOOLBAR_STOP_ID, ToolbarIcon, toolbar_icon_button,
    toolbar_icon_color, toolbar_icon_glyph,
};
pub(in crate::native_app) use crate::native_app::audio::audio_settings::{
    AUDIO_ENGINE_PILL_ID, AUDIO_SETTINGS_POPUP_HEIGHT, GENERAL_SETTINGS_BUTTON_ID, VOLUME_SLIDER_ID,
};
pub(in crate::native_app) use crate::native_app::audio::audio_settings::{
    audio_settings_popover, top_status_bar,
};
pub(in crate::native_app) use crate::native_app::audio::sample_load_actions::{
    KEYBOARD_SAMPLE_LOAD_DEBOUNCE, UNCACHED_SAMPLE_LOAD_DEBOUNCE,
};
pub(in crate::native_app) use crate::native_app::library_browser::file_actions::{
    format_copy_path, normalize_wav_file_in_place,
};
pub(in crate::native_app) use crate::native_app::library_browser::folder_browser::{
    DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
};
pub(in crate::native_app) use crate::native_app::library_browser::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
};
pub(in crate::native_app) use crate::native_app::shell::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_WINDOW_TITLE, debug_layout_requested,
};
pub(in crate::native_app) use crate::native_app::waveform::{
    WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID,
};
pub(in crate::native_app) use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, ResolvedOutput,
};
pub(in crate::native_app) use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};
