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
    time::{Duration, Instant},
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
};
use wavecrate::sample_sources::SampleCollection;
use wavecrate::sample_sources::config::AppSettingsCore;

use super::context_menu::{BrowserContextMenu, BrowserContextTargetKind};
use super::folder_browser::{
    FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanResult,
};
use super::metadata_tags::{MetadataTagInputMode, MetadataTagPersistResult};
use super::source_watcher::GuiSourceWatcherHandle;
use super::transaction_history::TransactionHistory;
use super::waveform::{WaveformFile, WaveformInteraction, WaveformState};

pub(in crate::gui_app) const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
pub(in crate::gui_app) const MIN_FOLDER_WIDTH: f32 = 180.0;
pub(in crate::gui_app) const MAX_FOLDER_WIDTH: f32 = 420.0;
pub(in crate::gui_app) const FOLDER_TREE_LIST_ID: u64 = 29_000;
pub(in crate::gui_app) const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize = 2;
pub(in crate::gui_app) const FOLDER_TREE_OVERSCAN_ROWS: usize = 4;
pub(in crate::gui_app) const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize = 96;
pub(in crate::gui_app) const SAMPLE_BROWSER_LIST_ID: u64 = 30_000;
pub(in crate::gui_app) const SAMPLE_BROWSER_ROW_HEIGHT: f32 = 22.0;
pub(in crate::gui_app) const SAMPLE_BROWSER_EDGE_CONTEXT_ROWS: usize = 2;
pub(in crate::gui_app) const SAMPLE_BROWSER_OVERSCAN_ROWS: usize = 4;
pub(in crate::gui_app) const SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS: usize = 128;
#[cfg(test)]
pub(in crate::gui_app) const DEFAULT_VOLUME: f32 = 1.0;
pub(in crate::gui_app) const VOLUME_SLIDER_ID: u64 = 31_000;
pub(in crate::gui_app) const VOLUME_SLIDER_WIDTH: f32 = 92.0;
pub(in crate::gui_app) const VOLUME_SLIDER_HEIGHT: f32 = 14.0;
pub(in crate::gui_app) const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);
pub(in crate::gui_app) const AUDIO_ENGINE_PILL_ID: u64 = 31_100;
pub(in crate::gui_app) const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
pub(in crate::gui_app) const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
pub(in crate::gui_app) const GENERAL_SETTINGS_BUTTON_ID: u64 = 31_110;
pub(in crate::gui_app) const GENERAL_SETTINGS_BUTTON_WIDTH: f32 = 28.0;
pub(in crate::gui_app) const GENERAL_SETTINGS_BUTTON_HEIGHT: f32 = 24.0;
pub(in crate::gui_app) const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 520.0;
pub(in crate::gui_app) const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 380.0;
pub(in crate::gui_app) const TRANSACTION_LIST_MODAL_ID: u64 = 31_200;
pub(in crate::gui_app) const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
pub(in crate::gui_app) const DRAG_PREVIEW_HEIGHT: f32 = 20.0;
pub(in crate::gui_app) const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
pub(in crate::gui_app) const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;
pub(in crate::gui_app) const WAVEFORM_SIGNAL_WIDGET_ID: u64 = 11;
pub(in crate::gui_app) const WAVEFORM_WIDGET_ID: u64 = 12;
pub(in crate::gui_app) const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration =
    Duration::from_millis(120);
pub(in crate::gui_app) const UNCACHED_SAMPLE_LOAD_DEBOUNCE: Duration = Duration::from_millis(90);
pub(in crate::gui_app) const KEYBOARD_SAMPLE_LOAD_DEBOUNCE: Duration = Duration::from_millis(650);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum AudioSettingsDropdown {
    Backend,
    Output,
    SampleRate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_app) enum AppSettingsTab {
    General,
    #[default]
    AudioEngine,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) enum GuiMessage {
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
    },
    SampleLoadProgress(ui::TaskTicket, f32),
    SampleLoadFinished(ui::TaskCompletion<SampleLoadResult>),
    WaveformCacheIndicatorRefreshFinished(ui::TaskTicket),
    WaveformCacheWarmFinished(ui::TaskTicket),
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
    ResolveFileMoveConflict(super::folder_browser::FileMoveConflictResolution),
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
pub(in crate::gui_app) struct SampleLoadResult {
    pub(in crate::gui_app) path: String,
    pub(in crate::gui_app) result: Result<WaveformState, String>,
    pub(in crate::gui_app) autoplay: bool,
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct WaveformCacheEntry {
    pub(in crate::gui_app) byte_len: usize,
    pub(in crate::gui_app) file: Arc<WaveformFile>,
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct WaveformCacheWarmResult {
    pub(in crate::gui_app) loaded: Vec<(PathBuf, Arc<WaveformFile>)>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::gui_app) struct WaveformCacheIndicatorRefreshResult {
    pub(in crate::gui_app) probed_paths: Vec<PathBuf>,
    pub(in crate::gui_app) playback_ready_paths: HashSet<PathBuf>,
    pub(in crate::gui_app) warm_candidate_paths: HashSet<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct NormalizationProgress {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) completed: usize,
    pub(in crate::gui_app) total: usize,
    pub(in crate::gui_app) detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct NormalizationResult {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) loaded_path: PathBuf,
    pub(in crate::gui_app) normalizing_loaded: bool,
    pub(in crate::gui_app) was_playing: bool,
    pub(in crate::gui_app) restart_ratio: f32,
    pub(in crate::gui_app) restart_span: Option<(f32, f32)>,
    pub(in crate::gui_app) normalized: Vec<PathBuf>,
    pub(in crate::gui_app) last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) struct PendingPlaybackStart {
    pub(in crate::gui_app) start_ratio: f32,
    pub(in crate::gui_app) end_ratio: f32,
    pub(in crate::gui_app) loop_offset_ratio: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum PendingSamplePlayback {
    RandomAudition { unit: f32 },
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.result.as_ref().err() == other.result.as_ref().err()
    }
}

pub(in crate::gui_app) struct GuiAppState {
    pub(in crate::gui_app) folder_panel: ui::PanelResizeState,
    pub(in crate::gui_app) folder_browser: FolderBrowserState,
    pub(in crate::gui_app) waveform: WaveformState,
    pub(in crate::gui_app) sample_status: String,
    pub(in crate::gui_app) worker_sender: Sender<GuiMessage>,
    pub(in crate::gui_app) worker_receiver: Option<Receiver<GuiMessage>>,
    pub(in crate::gui_app) next_task_id: u64,
    pub(in crate::gui_app) deferred_sample_load_task: ui::LatestTask,
    pub(in crate::gui_app) sample_load_task: ui::LatestTask,
    pub(in crate::gui_app) sample_load_cancel: Option<ui::CancellationToken>,
    pub(in crate::gui_app) audio_open_task: ui::LatestTask,
    pub(in crate::gui_app) audio_open_results:
        Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
    pub(in crate::gui_app) folder_progress: Option<FolderScanProgress>,
    pub(in crate::gui_app) pending_source_refreshes: HashSet<String>,
    pub(in crate::gui_app) source_watcher: Option<GuiSourceWatcherHandle>,
    pub(in crate::gui_app) normalization_progress: Option<NormalizationProgress>,
    pub(in crate::gui_app) progress_tick: f32,
    pub(in crate::gui_app) frame_cadence: ui::FrameCadenceMonitor,
    pub(in crate::gui_app) waveform_loading_progress: f32,
    pub(in crate::gui_app) waveform_loading_target_progress: f32,
    pub(in crate::gui_app) audio_player: Option<AudioPlayer>,
    pub(in crate::gui_app) loop_playback: bool,
    pub(in crate::gui_app) volume: f32,
    pub(in crate::gui_app) volume_persist_deadline: Option<Instant>,
    pub(in crate::gui_app) audio_output_config: AudioOutputConfig,
    pub(in crate::gui_app) audio_output_resolved: Option<ResolvedOutput>,
    pub(in crate::gui_app) audio_hosts: Vec<AudioHostSummary>,
    pub(in crate::gui_app) audio_devices: Vec<AudioDeviceSummary>,
    pub(in crate::gui_app) audio_sample_rates: Vec<u32>,
    pub(in crate::gui_app) persisted_settings: AppSettingsCore,
    pub(in crate::gui_app) audio_settings_open: bool,
    pub(in crate::gui_app) app_settings_tab: AppSettingsTab,
    pub(in crate::gui_app) audio_settings_dropdown: ui::ExclusiveOpen<AudioSettingsDropdown>,
    pub(in crate::gui_app) job_details_open: bool,
    pub(in crate::gui_app) transaction_list_open: bool,
    pub(in crate::gui_app) transaction_history: TransactionHistory<GuiAppState>,
    pub(in crate::gui_app) transaction_restoring: bool,
    pub(in crate::gui_app) context_menu: Option<BrowserContextMenu>,
    pub(in crate::gui_app) waveform_loading_label: Option<String>,
    pub(in crate::gui_app) audio_settings_error: Option<String>,
    pub(in crate::gui_app) current_playback_span: Option<(f32, f32)>,
    pub(in crate::gui_app) pending_playback_start: Option<PendingPlaybackStart>,
    pub(in crate::gui_app) pending_sample_playback: Option<PendingSamplePlayback>,
    pub(in crate::gui_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::gui_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
    pub(in crate::gui_app) metadata_tag_draft: String,
    pub(in crate::gui_app) metadata_tag_tokens: Vec<String>,
    pub(in crate::gui_app) metadata_tag_input_mode: MetadataTagInputMode,
    pub(in crate::gui_app) pending_metadata_tag_completion_query: Option<String>,
    pub(in crate::gui_app) metadata_tag_completion_cycle: ui::CyclicListSelectionCycle,
    pub(in crate::gui_app) metadata_tag_dictionary: BTreeMap<String, String>,
    pub(in crate::gui_app) metadata_tag_library_open: bool,
    pub(in crate::gui_app) metadata_tag_drag: Option<String>,
    pub(in crate::gui_app) metadata_tag_drop_hover: Option<String>,
    pub(in crate::gui_app) selected_metadata_tag: Option<String>,
    pub(in crate::gui_app) collapsed_metadata_tag_categories: HashSet<String>,
    pub(in crate::gui_app) metadata_tags_by_file: HashMap<String, Vec<String>>,
    pub(in crate::gui_app) sample_name_view_mode: SampleNameViewMode,
    pub(in crate::gui_app) startup_source_scan_pending: bool,
    pub(in crate::gui_app) startup_auto_load_pending: bool,
    pub(in crate::gui_app) waveform_cache: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::gui_app) waveform_cache_order: VecDeque<PathBuf>,
    pub(in crate::gui_app) waveform_cache_bytes: usize,
    pub(in crate::gui_app) waveform_cache_indicator_refresh_task: ui::LatestTask,
    pub(in crate::gui_app) waveform_cache_indicator_refresh_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheIndicatorRefreshResult>>>,
    pub(in crate::gui_app) waveform_cache_warm_pending: VecDeque<PathBuf>,
    pub(in crate::gui_app) waveform_cache_warm_task: ui::LatestTask,
    pub(in crate::gui_app) waveform_cache_warm_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheWarmResult>>>,
    pub(in crate::gui_app) cached_sample_paths: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum SampleNameViewMode {
    DiskFilename,
    MetadataLabel,
}

impl SampleNameViewMode {
    pub(in crate::gui_app) fn toggled(self) -> Self {
        match self {
            Self::DiskFilename => Self::MetadataLabel,
            Self::MetadataLabel => Self::DiskFilename,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct NativeFileDropHover {
    pub(in crate::gui_app) path: PathBuf,
    pub(in crate::gui_app) supported: bool,
}
