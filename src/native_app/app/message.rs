use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{path::PathBuf, time::Instant};
use wavecrate::sample_sources::{
    HarvestDerivationOperation, HarvestSeenPersistResult, SampleCollection,
    StarmapLayoutLoadResult, config::AppSettingsCore,
};
use wavecrate::selection::SelectionRange;
use wavecrate_analysis::aspects::SimilarityAspect;

use crate::native_app::app::ExtractedFilePlaybackType;
use crate::native_app::app::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmPlanResult,
    ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmResult, AppSettingsTab,
    AudioOpenTaskCompletion, FileMoveProgress, NormalizationProgress, NormalizationResult,
    SampleLoadPathValidation, SampleLoadResult, SamplePlaybackReady, StarmapViewportChange,
    WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
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
use crate::native_app::sample_library::native_file_open_actions::NativeAudioDocumentOpenValidation;
use crate::native_app::sample_library::similarity_prep::{
    SimilarityPrepEnqueueResult, SimilarityPrepStatusResult,
};
use crate::native_app::sample_library::similarity_scores::SimilarityScoresResult;
use crate::native_app::waveform::WaveformInteraction;
use crate::native_app::waveform::{SimilarSectionsResult, WaveformExtractionCompletion};
use crate::native_app::waveform_edits::WaveformDestructiveEditResult;

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
    NativeAudioDocumentOpenValidated {
        started_at: Instant,
        validation: NativeAudioDocumentOpenValidation,
    },
    DeferredSampleLoad {
        ticket: ui::TaskTicket,
        path: String,
        autoplay: bool,
        check_cache: bool,
        scheduled_at: Instant,
    },
    SampleLoadPathValidated {
        completion: ui::TaskCompletion<SampleLoadPathValidation>,
        started_at: Instant,
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
    PlayRandomListedSampleRange,
    PlayPreviousPlaybackHistory,
    PlayNextPlaybackHistory,
    ToggleStickyRandomSampleRangePlayback,
    LastPlayedPersistReady {
        ticket: ui::TaskTicket,
        request: LastPlayedPersistRequest,
    },
    LastPlayedPersisted(LastPlayedPersistResult),
    HarvestSeenPersisted(HarvestSeenPersistResult),
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
    StarmapLayoutLoaded(StarmapLayoutLoadResult),
    SimilarityPrepStatusResolved(SimilarityPrepStatusResult),
    SimilarityPrepEnqueueFinished(SimilarityPrepEnqueueResult),
    SimilarityScoresResolved(SimilarityScoresResult),
    Settings(SettingsMessage),
    Metadata(MetadataMessage),
    FocusLoadedFile,
    AdjustSelectedRatingWithoutAdvance(i8),
    AssignSelectedCollection(SampleCollection),
    RemoveContextSampleFromCollection,
    CleanMissingContextSampleFromCollection,
    CleanMissingFilesFromActiveCollection,
    MarkContextSampleHarvestDone,
    MarkContextSampleHarvestIgnored,
    ResetContextSampleHarvest,
    ToggleSelectedHarvestDone,
    ShowContextSampleHarvestOrigin,
    ShowContextSampleHarvestDerivatives,
    OpenContextSampleHarvestDestination,
    ShowSelectedSampleHarvestOrigin,
    ShowSelectedSampleHarvestDerivatives,
    OpenSelectedSampleHarvestDestination,
    NormalizeSelectedSamples,
    CopySelectedFiles,
    CutSelectedFiles,
    PasteCutFiles,
    DuplicateContextSampleSame,
    DuplicateContextSampleDouble,
    ContextSampleSameFinished {
        source_path: PathBuf,
        started_at: Instant,
        result: Result<wavecrate::sample_sources::ContextSampleSameResult, String>,
    },
    ContextSampleDoubleFinished {
        source_path: PathBuf,
        started_at: Instant,
        result: Result<wavecrate::sample_sources::ContextSampleDoubleResult, String>,
    },
    SelectedFilesCopyFinished {
        paths: Vec<PathBuf>,
        count: usize,
        started_at: Instant,
        result: Result<(), String>,
    },
    WaveformSelectionCopyExtracted {
        completion: WaveformExtractionCompletion,
        playback_type: ExtractedFilePlaybackType,
        started_at: Instant,
    },
    WaveformSelectionCopyFinished {
        source_path: PathBuf,
        selection: SelectionRange,
        copied_path: PathBuf,
        playback_type: ExtractedFilePlaybackType,
        source_duration_seconds: f64,
        started_at: Instant,
        result: Result<(), String>,
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
    UnlockContextSample,
    ToggleContextFolderLock,
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
    ToggleContextSourceProtection,
    SetContextSourcePrimary,
    ClearContextSourcePrimary,
    RemoveContextSource,
    CloseContextMenu,
    ToggleJobDetails,
    CloseJobDetails,
    ReleaseUpdateCheckFinished(
        ui::TaskCompletion<Result<Option<wavecrate::updater::PublicReleaseInfo>, String>>,
    ),
    OpenReleaseDownloadPage,
    ToggleShortcutHelp,
    CloseShortcutHelp,
    ToggleCurationFilterDropdown,
    CloseCurationFilterDropdown,
    ToggleHarvestFilterDropdown,
    CloseHarvestFilterDropdown,
    ToggleZeroCrossingSnap,
    ToggleBeatGuides,
    SetBeatGuideCount(u8),
    ChangeBeatGuideCountInput(String),
    CommitBeatGuideCountInput(String),
    ToggleMetronome,
    ToggleSimilarSections,
    SimilarSectionsResolved(SimilarSectionsResult),
    UndoTransaction,
    RedoTransaction,
    UndoTransactionsThrough(u64),
    RedoTransactionsThrough(u64),
    ToggleTransactionList,
    CloseTransactionList,
    FocusRenameInput(u64),
    FolderBrowserRenameFinished(RenameCommitCompletion),
    DeleteSelectedItem,
    RequestCropWaveformSelection,
    RequestTrimWaveformSelection,
    RequestReverseWaveformSelection,
    RequestMuteWaveformSelection,
    RequestExtractAndTrimWaveformSelection,
    RequestCropPlaymarkSelection,
    RequestTrimPlaymarkSelection,
    RequestReversePlaymarkSelection,
    RequestExtractAndTrimPlaymarkSelection,
    RequestApplyEditSelectionEffects,
    OpenContextMenu,
    ConfirmPendingWaveformDestructiveEdit,
    CancelPendingWaveformDestructiveEdit,
    WaveformDestructiveEditFinished(ui::TaskCompletion<WaveformDestructiveEditResult>),
    ExtractPlaymarkedRange,
    ExtractPlaymarkedRangeToHarvestDestination,
    PlaySelectionExtractionFinished {
        completion: WaveformExtractionCompletion,
        drag_position: Option<Point>,
        playback_type: ExtractedFilePlaybackType,
        harvest_operation: HarvestDerivationOperation,
        focus_derivative: bool,
        started_at: Instant,
    },
    SelectedWholeFilesHarvestExtractionFinished {
        started_at: Instant,
        result: wavecrate::sample_sources::WholeFileHarvestExtractionResult,
    },
    NavigateBrowser {
        delta: i32,
        extend: bool,
        preserve_selection: bool,
    },
    ToggleSelectedSampleAndAdvance,
    SelectAllSamples,
    ToggleRandomNavigationMode,
    ToggleSampleBrowserMapView,
    FocusSelectedStarmapNode,
    ChangeStarmapViewport(StarmapViewportChange),
    BeginStarmapAuditionDrag {
        path: Option<String>,
        position: Point,
        modifiers: PointerModifiers,
    },
    UpdateStarmapAuditionDrag {
        paths: Vec<String>,
        position: Point,
        modifiers: PointerModifiers,
    },
    AdvanceStarmapAudition {
        ticket: ui::TaskTicket,
    },
    FinishStarmapAuditionDrag,
    SampleBrowserWindowChanged(ui::VirtualListWindowChange),
    FolderTreeWindowChanged(ui::VirtualListWindowChange),
    CollapseSelectedFolder,
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
    #[cfg(test)]
    ToggleMetadataTagForFiles {
        tag: String,
        file_ids: Vec<String>,
    },
    DragMetadataTag {
        tag: String,
        drag: DragHandleMessage,
    },
    HoverMetadataTagDropCategory {
        category_id: String,
    },
    ClearMetadataTagDropCategoryUnless {
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
    SetNormalizedAuditionEnabled(bool),
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
    SetRatingDecayWeeks(u16),
    PickTrashFolder,
    ClearTrashFolder,
    ClearRebuildableCaches,
}
