pub(in crate::native_app) use crate::native_app::app::{
    AppSettingsTab, AudioAppState, AudioSettingsDropdown, BackgroundTaskState, ChromeUiState,
    DEFAULT_VOLUME, GuiMessage, LibraryAppState, MetadataAppState, MetadataMessage, NativeAppState,
    NativeFileDropHover, NormalizationProgress, PendingSamplePlayback, SampleLoadResult,
    SamplePlaybackReady, SettingsAppState, StartupState, StatusState, UiAppState, WaveformAppState,
    default_gui_shortcuts, format_sample_rate_label, view,
};
use crate::native_app::sample_library::folder_browser::view_contract::DEFAULT_FOLDER_WIDTH;
pub(in crate::native_app) use crate::native_app::sample_library::folder_browser::{
    FolderBrowserState, commands::FolderBrowserMessage, scan::FolderScanProgress,
};
pub(in crate::native_app) use crate::native_app::waveform::{WaveformInteraction, WaveformState};
use wavecrate::sample_sources::config::AppSettingsCore;

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
            ui: UiAppState::new(
                ChromeUiState::new(DEFAULT_FOLDER_WIDTH),
                StatusState::new(self.sample_status),
                SettingsAppState::new(self.persisted_settings.clone()),
                StartupState::new(false, false, false),
            ),
            library: LibraryAppState::new(self.folder_browser, None),
            waveform: WaveformAppState::new(
                self.waveform.unwrap_or_else(|| {
                    WaveformState::load_default().expect("default waveform state")
                }),
            ),
            background: BackgroundTaskState::for_tests(),
            audio: AudioAppState::for_tests(),
            transactions: Default::default(),
            metadata: MetadataAppState::from_settings(&self.persisted_settings),
        }
    }
}
