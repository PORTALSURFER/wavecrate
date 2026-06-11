use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{path::PathBuf, time::Instant};
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{
    ActiveFolderCacheWarmResult, AppSettingsTab, NormalizationProgress, NormalizationResult,
    SampleLoadResult, SamplePlaybackReady,
};
use crate::native_app::metadata::MetadataTagPersistResult;
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;
use crate::native_app::sample_library::folder_browser::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::commands::FileMoveConflictResolution;
use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanDiscoveryBatch, FolderScanProgress, FolderScanResult,
};
use crate::native_app::waveform::WaveformInteraction;

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
    StartupFolderVerifyFinished(ui::TaskTicket),
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
    WaveformCacheIndicatorRefreshFinished(ui::TaskTicket),
    WaveformCacheWarmFinished(ui::TaskTicket),
    ActiveFolderCacheWarmReady(ui::TaskTicket),
    ActiveFolderCacheWarmFinished(ui::TaskCompletion<ActiveFolderCacheWarmResult>),
    AudioPlayerOpenFinished(ui::TaskTicket),
    PlaySelectedSample,
    PlayRandomSampleRange,
    StopPlayback,
    ToggleLoopPlayback,
    Settings(SettingsMessage),
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
    FocusLoadedFile,
    AdjustSelectedRating(i8),
    AssignSelectedCollection(SampleCollection),
    RemoveContextSampleFromCollection,
    NormalizeSelectedSamples,
    CopySelectedFiles,
    ResolveFileMoveConflict(FileMoveConflictResolution),
    CancelFileMoveConflicts,
    CopyContextPath,
    OpenContextTarget,
    MoveContextTargetToTrash,
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
    DeleteSelectedItem,
    ExtractPlaymarkedRange,
    NavigateBrowser {
        delta: i32,
        extend: bool,
    },
    ToggleSelectedSampleAndAdvance,
    SelectAllSamples,
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
