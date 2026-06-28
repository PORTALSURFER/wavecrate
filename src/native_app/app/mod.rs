mod cache;
mod drop;
mod loading;
mod message;
mod progress;
mod settings;
mod state;

pub(in crate::native_app) use cache::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmPlanResult,
    ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmRequest, ActiveFolderCacheWarmResult,
    ActiveFolderCacheWarmStage, WaveformCacheEntry, WaveformCacheIndicatorRefreshResult,
    WaveformCacheWarmResult,
};
pub(in crate::native_app) use drop::NativeFileDropHover;
pub(in crate::native_app) use loading::{
    PendingSamplePlayback, SampleLoadResult, SampleLoadTaskCompletion, SamplePlaybackReady,
    SampleSelectionLoadState,
};
pub(in crate::native_app) use message::{
    GuiMessage, MetadataMessage, SettingsMessage, SimilaritySettingsPersistResult,
    SourceFilesystemSyncResult, TrashMoveTarget, VolumeSettingsPersistResult,
};
pub(in crate::native_app) use progress::{
    FileMoveProgress, NormalizationFailure, NormalizationHarvestDerivation, NormalizationProgress,
    NormalizationQueueItem, NormalizationResult,
};
pub(in crate::native_app) use settings::{
    AppSettingsTab, AudioSettingsDropdown, SampleNameViewMode,
};
#[cfg(test)]
pub(in crate::native_app) use state::DEFAULT_VOLUME;
#[cfg(test)]
pub(in crate::native_app) use state::ReleaseUpdateStatus;
pub(in crate::native_app) use state::{
    AudioAppState, AudioOpenCompletion, AudioOpenTaskCompletion, BackgroundTaskState,
    ChromeUiState, ClipboardHandoffTarget, CutFileClipboard, ExtractedFilePlaybackType,
    FolderScanWorkerEvent, LibraryAppState, MAX_BEAT_GUIDE_COUNT, MIN_BEAT_GUIDE_COUNT,
    MetadataAppState, NativeAppState, PendingFolderDelete, PendingPlaySelectionRetargetCycle,
    PendingPlaybackStart, PendingRuntimePlaybackStart, PendingWaveformDestructiveEdit,
    SampleBrowserDisplayMode, SettingsAppState, SourceFilesystemChangePlan, SourceRefreshRequest,
    SourceScanFinish, StarmapAuditionDragState, StarmapViewport, StarmapViewportChange,
    StartupState, StatusState, UiAppState, WaveformAppState, WaveformDestructiveEditKind,
    WaveformDestructiveEditPrompt, WaveformDestructiveEditUiContext, WaveformEditSelectionSnapshot,
    WaveformPlaySelectionSnapshot, run_folder_scan_worker,
};

pub(super) use crate::native_app::app_chrome::scene::view;
pub(super) use crate::native_app::audio::sample_load_actions::{
    NormalizedWaveformReload, SampleLoadPathValidation, WaveformPlaybackResume,
};
pub(super) use crate::native_app::sample_library::file_actions::sample_path_label;
pub(super) use crate::native_app::sample_library::folder_browser::FolderBrowserState;
pub(super) use crate::native_app::sample_library::folder_browser::commands::{
    FileMoveConflictResolution, FileMoveConflictResolutionRequest, FolderBrowserMessage,
};
pub(super) use crate::native_app::sample_library::folder_browser::scan::FolderScanProgress;
pub(super) use crate::native_app::shell::emit_gui_action;
pub(super) use crate::native_app::shell::shortcuts::{
    ShortcutHelpItem, ShortcutHelpSection, default_gui_shortcuts, shortcut_help_sections,
};
pub(super) use crate::native_app::ui::display::format_sample_rate_label;
pub(super) use crate::native_app::waveform::{
    WaveformActiveDragKind, WaveformContextMenu, WaveformInteraction, WaveformSelectionKind,
    WaveformState,
};
pub(super) use wavecrate::logging;
