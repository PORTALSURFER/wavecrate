mod cache;
mod drop;
mod loading;
mod message;
mod progress;
mod settings;
mod state;

pub(in crate::native_app) use cache::{
    ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmResult, ActiveFolderCacheWarmStage,
    WaveformCacheEntry, WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
};
pub(in crate::native_app) use drop::NativeFileDropHover;
pub(in crate::native_app) use loading::{
    PendingSamplePlayback, SampleLoadResult, SampleLoadTaskCompletion, SamplePlaybackReady,
    SampleSelectionLoadState,
};
pub(in crate::native_app) use message::{
    GuiMessage, MetadataMessage, SettingsMessage, SimilaritySettingsPersistResult, TrashMoveTarget,
    VolumeSettingsPersistResult,
};
pub(in crate::native_app) use progress::{
    NormalizationFailure, NormalizationProgress, NormalizationQueueItem, NormalizationResult,
};
pub(in crate::native_app) use settings::{
    AppSettingsTab, AudioSettingsDropdown, SampleNameViewMode,
};
#[cfg(test)]
pub(in crate::native_app) use state::DEFAULT_VOLUME;
pub(in crate::native_app) use state::{
    AudioAppState, AudioOpenCompletion, AudioOpenTaskCompletion, BackgroundTaskState,
    ChromeUiState, LibraryAppState, MetadataAppState, NativeAppState, PendingFolderDelete,
    PendingRuntimePlaybackStart, SettingsAppState, SourceFilesystemChangePlan,
    SourceRefreshRequest, SourceScanFinish, StartupState, StatusState, UiAppState,
    WaveformAppState, run_folder_scan_worker,
};

pub(super) use crate::native_app::app_chrome::scene::view;
pub(super) use crate::native_app::audio::sample_load_actions::{
    NormalizedWaveformReload, WaveformPlaybackResume,
};
pub(super) use crate::native_app::sample_library::file_actions::sample_path_label;
pub(super) use crate::native_app::sample_library::folder_browser::FolderBrowserState;
pub(super) use crate::native_app::sample_library::folder_browser::commands::{
    FileMoveConflictResolution, FileMoveConflictResolutionRequest, FolderBrowserMessage,
};
pub(super) use crate::native_app::sample_library::folder_browser::scan::FolderScanProgress;
pub(super) use crate::native_app::shell::emit_gui_action;
pub(super) use crate::native_app::shell::shortcuts::default_gui_shortcuts;
pub(super) use crate::native_app::ui::display::format_sample_rate_label;
pub(super) use crate::native_app::waveform::{
    WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState,
};
pub(super) use wavecrate::logging;
