//! Test-only native-app accessors.

pub(in crate::native_app) use super::app_scope::*;
pub(in crate::native_app) use super::audio_settings::{audio_settings_popover, top_status_bar};
pub(in crate::native_app) use super::context_menu::{BrowserContextMenu, BrowserContextTargetKind};
pub(in crate::native_app) use super::file_actions::{
    format_copy_path, normalize_wav_file_in_place,
};
pub(in crate::native_app) use super::launch::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, debug_layout_requested,
};
pub(in crate::native_app) use super::metadata_tags::MetadataTagInputMode;
pub(in crate::native_app) use super::sample_browser_view::SampleFileHitTarget;
pub(in crate::native_app) use super::sample_browser_view::sample_browser;
pub(in crate::native_app) use super::state::DEFAULT_VOLUME;
pub(in crate::native_app) use super::toolbar::{
    TOOLBAR_FOCUS_LOADED_ID, TOOLBAR_RANDOM_ID, TOOLBAR_STOP_ID, ToolbarIcon, toolbar_icon_button,
    toolbar_icon_color, toolbar_icon_glyph,
};
pub(in crate::native_app) use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, ResolvedOutput,
};
pub(in crate::native_app) use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};
