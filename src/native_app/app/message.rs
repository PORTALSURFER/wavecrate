use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{path::PathBuf, time::Instant};
use wavecrate::sample_sources::{SampleCollection, config::AppSettingsCore};
use wavecrate::selection::SelectionRange;
use wavecrate_analysis::aspects::SimilarityAspect;

use crate::native_app::app::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmPlanResult,
    ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmResult, AppSettingsTab,
    AudioOpenTaskCompletion, FileMoveProgress, NormalizationProgress, NormalizationResult,
    SampleLoadResult, SamplePlaybackReady, WaveformCacheIndicatorRefreshResult,
    WaveformCacheWarmResult,
};
use crate::native_app::audio::playback_history::{
    LastPlayedPersistRequest, LastPlayedPersistResult,
};
use crate::native_app::metadata::MetadataTagPersistResult;
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::commands::RenameCommitCompletion;
use crate::native_app::sample_library::folder_browser::commands::{
    FileMoveConflictCompletion, FileMoveConflictResolutionRequest, FolderMoveCompletion,
};
use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanDiscoveryBatch, FolderScanProgress, FolderScanResult, FolderTreeRefreshResult,
    FolderVerifyResult,
};
use crate::native_app::sample_library::similarity_prep::{
    SimilarityPrepEnqueueResult, SimilarityPrepStatusResult,
};
use crate::native_app::sample_library::similarity_scores::SimilarityScoresResult;
use crate::native_app::waveform::WaveformExtractionCompletion;
use crate::native_app::waveform::WaveformInteraction;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum TrashMoveTarget {
    Folder(PathBuf),
    Files(Vec<PathBuf>),
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum GuiMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    AddSourceDialogFinished(ui::PlatformResult),
    ContextPathCopyFinished {
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: ui::PlatformResult,
    },
    TrashFolderDialogFinished(ui::PlatformResult),
    ContextTargetOpenFinished {
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: ui::PlatformResult,
    },
    FolderScanProgress(FolderScanProgress),
    FolderScanDiscoveryBatch(FolderScanDiscoveryBatch),
    FolderScanFinished(FolderScanResult),
    FolderTreeRefreshFinished(ui::TaskCompletion<FolderTreeRefreshResult>),
    SelectedFolderVerifyFinished(ui::TaskCompletion<FolderVerifyResult>),
    SourceFilesystemChanged {
        source_id: String,
        paths: Vec<PathBuf>,
        overflowed: bool,
    },
    SourceFilesystemSyncFinished(SourceFilesystemSyncResult),
    NormalizationProgress(NormalizationProgress),
    NormalizationFinished(NormalizationResult),
    SelectSampleWithModifiers {
        path: String,
        modifiers: PointerModifiers,
    },
    OpenSampleContextMenu {
        path: String,
        position: Point,
    },
    DragSampleFile {
        path: String,
        drag: DragHandleMessage,
    },
    ExternalDragCompleted(Result<ui::ExternalDragOutcome, String>),
    ExternalWaveformFileDropFinished {
        source: PathBuf,
        started_at: Instant,
        result: Result<PathBuf, String>,
    },
    DeferredSampleLoad {
        ticket: ui::TaskTicket,
        path: String,
        autoplay: bool,
        check_cache: bool,
        scheduled_at: Instant,
    },
    SampleLoadProgress(ui::ResourceKey, ui::TaskTicket, f32),
    SamplePlaybackReady(ui::KeyedTaskCompletion<ui::ResourceKey, SamplePlaybackReady>),
    SampleLoadFinished(ui::KeyedTaskCompletion<ui::ResourceKey, SampleLoadResult>),
    WaveformCacheIndicatorRefreshFinished(ui::TaskCompletion<WaveformCacheIndicatorRefreshResult>),
    WaveformCacheWarmFinished(ui::KeyedTaskCompletion<ui::ResourceKey, WaveformCacheWarmResult>),
    ActiveFolderCacheWarmPlanProgress(ui::TaskCompletion<ActiveFolderCacheWarmPlanProgress>),
    ActiveFolderCacheWarmPlanned(ui::TaskCompletion<ActiveFolderCacheWarmPlanResult>),
    ActiveFolderCacheWarmReady(ui::TaskTicket),
    ActiveFolderCacheWarmProgress(
        ui::KeyedTaskCompletion<ui::ResourceKey, ActiveFolderCacheWarmProgress>,
    ),
    ActiveFolderCacheWarmFinished(
        ui::KeyedTaskCompletion<ui::ResourceKey, ActiveFolderCacheWarmResult>,
    ),
    AudioPlayerOpenFinished(AudioOpenTaskCompletion),
    PlaySelectedSample,
    PlayFromCurrentPlayStart,
    PlayRandomSampleRange,
    ToggleStickyRandomSampleRangePlayback,
    LastPlayedPersistReady {
        ticket: ui::TaskTicket,
        request: LastPlayedPersistRequest,
    },
    LastPlayedPersisted(LastPlayedPersistResult),
    VolumeSettingsPersisted(VolumeSettingsPersistResult),
    StopPlayback,
    ToggleLoopPlayback,
    SetSimilarityAspectWeightingEnabled(bool),
    SetSimilarityAspectEnabled {
        aspect: SimilarityAspect,
        enabled: bool,
    },
    SetSimilarityAspectWeight {
        aspect: SimilarityAspect,
        weight: f32,
    },
    SimilaritySettingsPersisted(SimilaritySettingsPersistResult),
    SimilarityPrepStatusResolved(SimilarityPrepStatusResult),
    SimilarityPrepEnqueueFinished(SimilarityPrepEnqueueResult),
    SimilarityScoresResolved(SimilarityScoresResult),
    Settings(SettingsMessage),
    Metadata(MetadataMessage),
    FocusLoadedFile,
    AdjustSelectedRating(i8),
    AssignSelectedCollection(SampleCollection),
    RemoveContextSampleFromCollection,
    NormalizeSelectedSamples,
    CopySelectedFiles,
    SelectedFilesCopyFinished {
        count: usize,
        started_at: Instant,
        result: Result<(), String>,
    },
    WaveformSelectionCopyFinished {
        source_path: PathBuf,
        selection: SelectionRange,
        started_at: Instant,
        result: Result<PathBuf, String>,
    },
    FileMoveProgress(FileMoveProgress),
    SetFileMoveConflictApplyToRemaining(bool),
    ResolveFileMoveConflict(FileMoveConflictResolutionRequest),
    FolderMoveFinished {
        started_at: Instant,
        completion: FolderMoveCompletion,
    },
    FileMoveConflictFinished {
        started_at: Instant,
        completion: FileMoveConflictCompletion,
    },
    CancelFileMoveConflicts,
    CopyContextPath,
    OpenContextTarget,
    CreateFolderAtContextTarget,
    RenameContextFolder,
    ContextFolderCreateFinished {
        parent_id: String,
        started_at: Instant,
        result: Result<PathBuf, String>,
    },
    MoveContextTargetToTrash,
    RequestDeleteContextFolder,
    ConfirmContextFolderDelete,
    CancelContextFolderDelete,
    TrashMoveFinished {
        target: TrashMoveTarget,
        action: &'static str,
        started_at: Instant,
        result: Result<Vec<PathBuf>, String>,
    },
    RefreshContextSource,
    ProcessContextSource,
    RemoveContextSource,
    CloseContextMenu,
    ToggleJobDetails,
    CloseJobDetails,
    ToggleShortcutHelp,
    CloseShortcutHelp,
    ToggleBeatGuides,
    AdjustBeatGuideCount(i8),
    UndoTransaction,
    RedoTransaction,
    ToggleTransactionList,
    CloseTransactionList,
    FocusRenameInput(u64),
    FolderBrowserRenameFinished(RenameCommitCompletion),
    DeleteSelectedItem,
    RequestCropWaveformSelection,
    RequestTrimWaveformSelection,
    RequestExtractAndTrimWaveformSelection,
    ConfirmPendingWaveformDestructiveEdit,
    CancelPendingWaveformDestructiveEdit,
    ExtractPlaymarkedRange,
    PlaySelectionExtractionFinished {
        completion: WaveformExtractionCompletion,
        drag_position: Option<Point>,
        started_at: Instant,
    },
    NavigateBrowser {
        delta: i32,
        extend: bool,
        preserve_selection: bool,
    },
    ToggleSelectedSampleAndAdvance,
    SelectAllSamples,
    ToggleRandomNavigationMode,
    SampleBrowserWindowChanged(ui::VirtualListWindowChange),
    FolderTreeWindowChanged(ui::VirtualListWindowChange),
    CollapseSelectedFolder,
    ExpandSelectedFolder,
    CancelBrowserDragOnSampleList,
    DropWaveformSelectionOnSampleList,
    Waveform(WaveformInteraction),
    WaveformFileDrop(NativeFileDrop),
    Frame,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SourceFilesystemSyncResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) changed_count: usize,
    pub(in crate::native_app) result: Result<(), String>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct VolumeSettingsPersistResult {
    pub(in crate::native_app) persisted: AppSettingsCore,
    pub(in crate::native_app) result: Result<(), String>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SimilaritySettingsPersistResult {
    pub(in crate::native_app) persisted: AppSettingsCore,
    pub(in crate::native_app) result: Result<(), String>,
}

impl PartialEq for SimilaritySettingsPersistResult {
    fn eq(&self, other: &Self) -> bool {
        self.result == other.result && self.persisted.similarity == other.persisted.similarity
    }
}

impl PartialEq for VolumeSettingsPersistResult {
    fn eq(&self, other: &Self) -> bool {
        self.result == other.result
            && self.persisted.volume.to_bits() == other.persisted.volume.to_bits()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum MetadataMessage {
    FocusMetadataTagInput,
    MetadataTagInput(radiant::widgets::TextInputMessage),
    CancelMetadataTagEntry,
    MoveMetadataTagCompletion(i32),
    HoverMetadataTagCompletion(String),
    SelectMetadataTagCompletion(String),
    ToggleMetadataTagLibrary,
    ToggleMetadataTagCategory(String),
    SelectMetadataTag(String),
    ToggleMetadataTag(String),
    DragMetadataTag {
        tag: String,
        drag: DragHandleMessage,
    },
    HoverMetadataTagDropCategory {
        category_id: String,
    },
    DropMetadataTagOnCategory {
        category_id: String,
    },
    OpenMetadataTagContextMenu {
        tag: String,
        position: ui::Point,
    },
    DeleteContextMetadataTag,
    DeleteSelectedMetadataTag,
    MetadataTagsPersisted(MetadataTagPersistResult),
    ToggleSampleNameViewMode,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum SettingsMessage {
    SetVolume(f32),
    ToggleHelpTooltips,
    ToggleAudioSettings,
    OpenGeneralSettings,
    SelectSettingsTab(AppSettingsTab),
    CloseAudioSettings,
    ToggleAudioBackendDropdown,
    ToggleAudioOutputDropdown,
    ToggleAudioSampleRateDropdown,
    CloseAudioSettingsDropdowns,
    SetAudioOutputHost(Option<String>),
    SetAudioOutputDevice(Option<String>),
    SetAudioOutputSampleRate(Option<u32>),
    PickTrashFolder,
    ClearTrashFolder,
    ClearRebuildableCaches,
}
