use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{path::PathBuf, time::Instant};
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{
    ActiveFolderCacheWarmResult, AppSettingsTab, AudioOpenTaskCompletion, NormalizationProgress,
    NormalizationResult, SampleLoadResult, SamplePlaybackReady,
    WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
};
use crate::native_app::audio::playback_history::LastPlayedPersistResult;
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
    SampleLoadProgress(ui::TaskTicket, f32),
    SamplePlaybackReady(ui::TaskCompletion<SamplePlaybackReady>),
    SampleLoadFinished(ui::TaskCompletion<SampleLoadResult>),
    WaveformCacheIndicatorRefreshFinished(ui::TaskCompletion<WaveformCacheIndicatorRefreshResult>),
    WaveformCacheWarmFinished(ui::TaskCompletion<WaveformCacheWarmResult>),
    ActiveFolderCacheWarmReady(ui::TaskTicket),
    ActiveFolderCacheWarmFinished(ui::TaskCompletion<ActiveFolderCacheWarmResult>),
    AudioPlayerOpenFinished(AudioOpenTaskCompletion),
    PlaySelectedSample,
    PlayRandomSampleRange,
    LastPlayedPersisted(LastPlayedPersistResult),
    StopPlayback,
    ToggleLoopPlayback,
    PrepareSimilarityForSelectedSource,
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
    ContextFolderCreateFinished {
        parent_id: String,
        started_at: Instant,
        result: Result<PathBuf, String>,
    },
    MoveContextTargetToTrash,
    TrashMoveFinished {
        target: TrashMoveTarget,
        action: &'static str,
        started_at: Instant,
        result: Result<Vec<PathBuf>, String>,
    },
    RefreshContextSource,
    RemoveContextSource,
    CloseContextMenu,
    ToggleJobDetails,
    CloseJobDetails,
    UndoTransaction,
    RedoTransaction,
    ToggleTransactionList,
    CloseTransactionList,
    FocusRenameInput(u64),
    FolderBrowserRenameFinished(RenameCommitCompletion),
    DeleteSelectedItem,
    ExtractPlaymarkedRange,
    PlaySelectionExtractionFinished {
        completion: WaveformExtractionCompletion,
        drag_position: Option<Point>,
        started_at: Instant,
    },
    NavigateBrowser {
        delta: i32,
        extend: bool,
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
