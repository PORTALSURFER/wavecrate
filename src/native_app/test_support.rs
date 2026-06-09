//! Test-only native-app accessors.

pub(in crate::native_app) use super::metadata::MetadataTagInputMode;
pub(in crate::native_app) use super::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
pub(in crate::native_app) use crate::native_app::app::*;
pub(in crate::native_app) use crate::native_app::app_chrome::library_browser::sample_browser_view::SampleFileHitTarget;
pub(in crate::native_app) use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser_from_state as sample_browser;
pub(in crate::native_app) use crate::native_app::app_chrome::settings::{
    AUDIO_ENGINE_PILL_ID, AUDIO_SETTINGS_POPUP_HEIGHT, GENERAL_SETTINGS_BUTTON_ID, VOLUME_SLIDER_ID,
};
pub(in crate::native_app) use crate::native_app::app_chrome::settings::{
    audio_settings_popover, top_control_bar,
};
pub(in crate::native_app) use crate::native_app::app_chrome::toolbar::{
    TOOLBAR_FOCUS_LOADED_ID, TOOLBAR_RANDOM_ID, TOOLBAR_STOP_ID, ToolbarIcon, toolbar_icon_button,
    toolbar_icon_color, toolbar_icon_glyph,
};
pub(in crate::native_app) use crate::native_app::audio::sample_load_actions::{
    KEYBOARD_SAMPLE_LOAD_DEBOUNCE, UNCACHED_SAMPLE_LOAD_DEBOUNCE,
};
pub(in crate::native_app) use crate::native_app::sample_library::file_actions::{
    format_copy_path, normalize_wav_file_in_place,
};
pub(in crate::native_app) use crate::native_app::sample_library::folder_browser::{
    DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
};
pub(in crate::native_app) use crate::native_app::sample_library::sample_list::{
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

pub(in crate::native_app) struct NativeAppStateFixture {
    folder_browser: FolderBrowserState,
    waveform: Option<WaveformState>,
    sample_status: String,
    persisted_settings: AppSettingsCore,
}

impl Default for NativeAppStateFixture {
    fn default() -> Self {
        Self {
            folder_browser: FolderBrowserState::load_default(),
            waveform: None,
            sample_status: String::from("Select a sample to load"),
            persisted_settings: AppSettingsCore::default(),
        }
    }
}

impl NativeAppStateFixture {
    pub(in crate::native_app) fn with_synthetic_waveform(mut self) -> Self {
        self.waveform = Some(WaveformState::synthetic_for_tests());
        self
    }

    pub(in crate::native_app) fn with_sample_status(
        mut self,
        sample_status: impl Into<String>,
    ) -> Self {
        self.sample_status = sample_status.into();
        self
    }

    pub(in crate::native_app) fn build(self) -> NativeAppState {
        NativeAppState {
            chrome: ChromeUiState::new(DEFAULT_FOLDER_WIDTH),
            folder_browser: self.folder_browser,
            waveform: self
                .waveform
                .unwrap_or_else(|| WaveformState::load_default().expect("default waveform state")),
            sample_status: self.sample_status,
            background: BackgroundTaskState::for_tests(),
            folder_progress: None,
            pending_source_refreshes: Default::default(),
            source_watcher: None,
            waveform_load: WaveformLoadState::default(),
            audio: AudioAppState::for_tests(),
            persisted_settings: self.persisted_settings.clone(),
            settings_ui: SettingsUiState::default(),
            transaction_history: Default::default(),
            transaction_restoring: false,
            browser_interaction: BrowserInteractionState::default(),
            metadata: MetadataAppState::from_settings(&self.persisted_settings),
            startup_source_scan_pending: false,
            startup_folder_verify_pending: false,
            startup_auto_load_pending: false,
            waveform_cache: WaveformCacheState::default(),
        }
    }
}
