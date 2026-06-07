use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
};
use wavecrate::sample_sources::SampleCollection;
use wavecrate::sample_sources::config::AppSettingsCore;

use super::context_menu::{BrowserContextMenu, BrowserContextTargetKind};
use super::metadata_tags::{MetadataTagInputMode, MetadataTagPersistResult};
use super::transaction_history::TransactionHistory;
use super::waveform::{WaveformFile, WaveformInteraction, WaveformPlaybackReady, WaveformState};
use crate::native_app::browser::folder_browser::{
    FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanResult, FolderVerifyResult,
};
use crate::native_app::browser::source_watcher::GuiSourceWatcherHandle;

#[cfg(test)]
pub(in crate::native_app) const DEFAULT_VOLUME: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum AudioSettingsDropdown {
    Backend,
    Output,
    SampleRate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum AppSettingsTab {
    General,
    #[default]
    AudioEngine,
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
    MetadataTagInput(radiant::widgets::TextInputMessage),
    CancelMetadataTagEntry,
    MoveMetadataTagCompletion(i32),
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
    ClearRebuildableCaches,
    PickTrashFolder,
    ClearTrashFolder,
    FocusLoadedFile,
    AdjustSelectedRating(i8),
    AssignSelectedCollection(SampleCollection),
    RemoveContextSampleFromCollection,
    NormalizeSelectedSamples,
    CopySelectedFiles,
    ResolveFileMoveConflict(crate::native_app::browser::folder_browser::FileMoveConflictResolution),
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
    CollapseSelectedFolder,
    ExpandSelectedFolder,
    CancelBrowserDragOnSampleList,
    DropWaveformSelectionOnSampleList,
    Waveform(WaveformInteraction),
    NativeFileDrop(NativeFileDrop),
    Frame,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SampleLoadResult {
    pub(in crate::native_app) path: String,
    pub(in crate::native_app) result: Result<WaveformState, String>,
    pub(in crate::native_app) autoplay: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SamplePlaybackReady {
    pub(in crate::native_app) path: String,
    pub(in crate::native_app) audio: WaveformPlaybackReady,
    pub(in crate::native_app) autoplay: bool,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformCacheEntry {
    pub(in crate::native_app) byte_len: usize,
    pub(in crate::native_app) file: Arc<WaveformFile>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformCacheWarmResult {
    pub(in crate::native_app) loaded: Vec<(PathBuf, Arc<WaveformFile>)>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct ActiveFolderCacheWarmResult {
    pub(in crate::native_app) folder_id: String,
    pub(in crate::native_app) loaded: Vec<(PathBuf, Arc<WaveformFile>)>,
    pub(in crate::native_app) cancelled: bool,
}

#[derive(Clone, Debug, Default)]
pub(in crate::native_app) struct WaveformCacheIndicatorRefreshResult {
    pub(in crate::native_app) probed_paths: Vec<PathBuf>,
    pub(in crate::native_app) playback_ready_paths: HashSet<PathBuf>,
    pub(in crate::native_app) warm_candidate_paths: HashSet<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct NormalizationProgress {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct NormalizationResult {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) loaded_path: PathBuf,
    pub(in crate::native_app) normalizing_loaded: bool,
    pub(in crate::native_app) was_playing: bool,
    pub(in crate::native_app) restart_ratio: f32,
    pub(in crate::native_app) restart_span: Option<(f32, f32)>,
    pub(in crate::native_app) normalized: Vec<PathBuf>,
    pub(in crate::native_app) last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PendingPlaybackStart {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) loop_offset_ratio: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum PendingSamplePlayback {
    RandomAudition { unit: f32 },
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.result.as_ref().err() == other.result.as_ref().err()
    }
}

pub(in crate::native_app) struct NativeAppState {
    pub(in crate::native_app) folder_panel: ui::PanelResizeState,
    pub(in crate::native_app) folder_browser: FolderBrowserState,
    pub(in crate::native_app) waveform: WaveformState,
    pub(in crate::native_app) sample_status: String,
    pub(in crate::native_app) worker_sender: Sender<GuiMessage>,
    pub(in crate::native_app) worker_receiver: Option<Receiver<GuiMessage>>,
    pub(in crate::native_app) next_task_id: u64,
    pub(in crate::native_app) deferred_sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) audio_open_task: ui::LatestTask,
    pub(in crate::native_app) audio_open_results:
        Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
    pub(in crate::native_app) folder_progress: Option<FolderScanProgress>,
    pub(in crate::native_app) pending_source_refreshes: HashSet<String>,
    pub(in crate::native_app) source_watcher: Option<GuiSourceWatcherHandle>,
    pub(in crate::native_app) startup_folder_verify_task: ui::LatestTask,
    pub(in crate::native_app) startup_folder_verify_results:
        Arc<Mutex<HashMap<ui::TaskTicket, FolderVerifyResult>>>,
    pub(in crate::native_app) normalization_progress: Option<NormalizationProgress>,
    pub(in crate::native_app) progress_tick: f32,
    pub(in crate::native_app) frame_cadence: ui::FrameCadenceMonitor,
    pub(in crate::native_app) waveform_loading_progress: f32,
    pub(in crate::native_app) waveform_loading_target_progress: f32,
    pub(in crate::native_app) audio_player: Option<AudioPlayer>,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) volume: f32,
    pub(in crate::native_app) volume_persist_deadline: Option<Instant>,
    pub(in crate::native_app) audio_output_config: AudioOutputConfig,
    pub(in crate::native_app) audio_output_resolved: Option<ResolvedOutput>,
    pub(in crate::native_app) audio_hosts: Vec<AudioHostSummary>,
    pub(in crate::native_app) audio_devices: Vec<AudioDeviceSummary>,
    pub(in crate::native_app) audio_sample_rates: Vec<u32>,
    pub(in crate::native_app) persisted_settings: AppSettingsCore,
    pub(in crate::native_app) audio_settings_open: bool,
    pub(in crate::native_app) app_settings_tab: AppSettingsTab,
    pub(in crate::native_app) audio_settings_dropdown: ui::ExclusiveOpen<AudioSettingsDropdown>,
    pub(in crate::native_app) job_details_open: bool,
    pub(in crate::native_app) transaction_list_open: bool,
    pub(in crate::native_app) transaction_history: TransactionHistory<NativeAppState>,
    pub(in crate::native_app) transaction_restoring: bool,
    pub(in crate::native_app) context_menu: Option<BrowserContextMenu>,
    pub(in crate::native_app) waveform_loading_label: Option<String>,
    pub(in crate::native_app) audio_settings_error: Option<String>,
    pub(in crate::native_app) current_playback_span: Option<(f32, f32)>,
    pub(in crate::native_app) pending_playback_start: Option<PendingPlaybackStart>,
    pub(in crate::native_app) pending_sample_playback: Option<PendingSamplePlayback>,
    pub(in crate::native_app) early_sample_playback_path: Option<String>,
    pub(in crate::native_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::native_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
    pub(in crate::native_app) metadata_tag_draft: String,
    pub(in crate::native_app) metadata_tag_tokens: Vec<String>,
    pub(in crate::native_app) metadata_tag_input_mode: MetadataTagInputMode,
    pub(in crate::native_app) pending_metadata_tag_completion_query: Option<String>,
    pub(in crate::native_app) metadata_tag_completion_cycle: ui::CyclicListSelectionCycle,
    pub(in crate::native_app) metadata_tag_dictionary: BTreeMap<String, String>,
    pub(in crate::native_app) metadata_tag_library_open: bool,
    pub(in crate::native_app) metadata_tag_drag: Option<String>,
    pub(in crate::native_app) metadata_tag_drop_hover: Option<String>,
    pub(in crate::native_app) selected_metadata_tag: Option<String>,
    pub(in crate::native_app) collapsed_metadata_tag_categories: HashSet<String>,
    pub(in crate::native_app) metadata_tags_by_file: HashMap<String, Vec<String>>,
    pub(in crate::native_app) sample_name_view_mode: SampleNameViewMode,
    pub(in crate::native_app) startup_source_scan_pending: bool,
    pub(in crate::native_app) startup_folder_verify_pending: bool,
    pub(in crate::native_app) startup_auto_load_pending: bool,
    pub(in crate::native_app) waveform_cache: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::native_app) waveform_cache_order: VecDeque<PathBuf>,
    pub(in crate::native_app) waveform_cache_bytes: usize,
    pub(in crate::native_app) waveform_cache_indicator_refresh_task: ui::LatestTask,
    pub(in crate::native_app) waveform_cache_indicator_refresh_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheIndicatorRefreshResult>>>,
    pub(in crate::native_app) waveform_cache_warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) waveform_cache_warm_task: ui::LatestTask,
    pub(in crate::native_app) waveform_cache_warm_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheWarmResult>>>,
    pub(in crate::native_app) active_folder_cache_warm_delay_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_cache_warm_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_cache_warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_cache_warm_folder_id: Option<String>,
    pub(in crate::native_app) active_folder_cache_warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) cached_sample_paths: HashSet<String>,
}

impl PartialEq for ActiveFolderCacheWarmResult {
    fn eq(&self, other: &Self) -> bool {
        self.folder_id == other.folder_id
            && self.cancelled == other.cancelled
            && self
                .loaded
                .iter()
                .map(|(path, _)| path)
                .eq(other.loaded.iter().map(|(path, _)| path))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SampleNameViewMode {
    DiskFilename,
    MetadataLabel,
}

impl SampleNameViewMode {
    pub(in crate::native_app) fn toggled(self) -> Self {
        match self {
            Self::DiskFilename => Self::MetadataLabel,
            Self::MetadataLabel => Self::DiskFilename,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct NativeFileDropHover {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) supported: bool,
}
