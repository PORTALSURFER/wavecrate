use radiant::prelude as ui;
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
use wavecrate::sample_sources::config::AppSettingsCore;

use super::{
    AppSettingsTab, AudioSettingsDropdown, GuiMessage, NativeFileDropHover, NormalizationProgress,
    PendingPlaybackStart, PendingSamplePlayback, SampleNameViewMode, WaveformCacheEntry,
    WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
};
use crate::native_app::metadata::MetadataTagInputMode;
use crate::native_app::sample_library::context_menu_target::BrowserContextMenu;
use crate::native_app::sample_library::folder_browser::{
    FolderBrowserState, FolderScanProgress, FolderVerifyResult,
};
use crate::native_app::sample_library::source_watcher::GuiSourceWatcherHandle;
use crate::native_app::transaction_history::NativeTransactionHistory;
use crate::native_app::waveform::WaveformState;

#[cfg(test)]
pub(in crate::native_app) const DEFAULT_VOLUME: f32 = 1.0;

pub(in crate::native_app) struct NativeAppState {
    // OPT-496 ownership map:
    //
    // Extraction order should keep task plumbing first, then audio and waveform,
    // then metadata, chrome/settings, tests, and finally transactions. The main
    // risk points are stale async task results, playback startup ordering,
    // waveform cache warming, metadata-driven browser filtering, and undo/redo
    // closures that currently target the concrete root state.
    //
    // ChromeUiState owns layout chrome and top-level modal/transient flags.
    pub(in crate::native_app) chrome: ChromeUiState,

    // LibraryAppState owns source/folder/sample browsing, source refresh and
    // watcher state, scan progress, startup source scan flags, context-menu
    // targets, file-drop hover, internal drag paths, and sample_status.
    pub(in crate::native_app) folder_browser: FolderBrowserState,

    // WaveformAppState owns core waveform interaction state.
    pub(in crate::native_app) waveform: WaveformState,
    pub(in crate::native_app) sample_status: String,

    // BackgroundTaskState owns generic GUI task transport, ticket allocation,
    // latest-task trackers, cancellation handles, task result maps, progress
    // tick/cadence state, and startup folder verification task plumbing.
    pub(in crate::native_app) background: BackgroundTaskState,
    pub(in crate::native_app) folder_progress: Option<FolderScanProgress>,
    pub(in crate::native_app) pending_source_refreshes: HashSet<String>,
    pub(in crate::native_app) source_watcher: Option<GuiSourceWatcherHandle>,

    // WaveformLoadState owns sample-load visible progress, loading label, and
    // input-blocking target progress. WaveformCacheState owns waveform cache
    // entries, LRU accounting, cache indicator refresh, persisted cache warming,
    // active-folder cache warming, and cached path lookups.
    pub(in crate::native_app) waveform_load: WaveformLoadState,
    pub(in crate::native_app) waveform_cache: WaveformCacheState,

    // AudioAppState owns player/runtime playback state, output configuration,
    // resolved device state, discovered hosts/devices/rates, volume persistence,
    // pending playback coordination, and audio-domain settings errors.
    pub(in crate::native_app) audio: AudioAppState,
    pub(in crate::native_app) persisted_settings: AppSettingsCore,

    // SettingsUiState owns settings-window presentation state only. Durable
    // settings values and audio-device errors stay with their domain owners.
    pub(in crate::native_app) settings_ui: SettingsUiState,

    // TransactionState owns history and restore guards. Actions execute through
    // TransactionContext rather than arbitrary closures over the root state.
    pub(in crate::native_app) transaction_history: NativeTransactionHistory,
    pub(in crate::native_app) transaction_restoring: bool,
    pub(in crate::native_app) browser_interaction: BrowserInteractionState,

    // MetadataAppState owns tag entry, completion, dictionary, library panel,
    // drag/drop, selection, collapsed categories, per-file tag assignments, and
    // sample-name view mode used by metadata display.
    pub(in crate::native_app) metadata: MetadataAppState,
    pub(in crate::native_app) startup_source_scan_pending: bool,
    pub(in crate::native_app) startup_folder_verify_pending: bool,
    pub(in crate::native_app) startup_auto_load_pending: bool,
}

pub(in crate::native_app) struct ChromeUiState {
    pub(in crate::native_app) folder_panel: ui::PanelResizeState,
    pub(in crate::native_app) job_details_open: bool,
    pub(in crate::native_app) transaction_list_open: bool,
}

impl ChromeUiState {
    pub(in crate::native_app) fn new(folder_width: f32) -> Self {
        Self {
            folder_panel: ui::PanelResizeState::new(folder_width),
            job_details_open: false,
            transaction_list_open: false,
        }
    }
}

pub(in crate::native_app) struct SettingsUiState {
    pub(in crate::native_app) audio_settings_open: bool,
    pub(in crate::native_app) app_settings_tab: AppSettingsTab,
    pub(in crate::native_app) audio_settings_dropdown: ui::ExclusiveOpen<AudioSettingsDropdown>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            audio_settings_open: false,
            app_settings_tab: Default::default(),
            audio_settings_dropdown: ui::ExclusiveOpen::new(),
        }
    }
}

pub(in crate::native_app) struct BrowserInteractionState {
    pub(in crate::native_app) context_menu: Option<BrowserContextMenu>,
    pub(in crate::native_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::native_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
}

impl Default for BrowserInteractionState {
    fn default() -> Self {
        Self {
            context_menu: None,
            native_file_drop_hover: None,
            pending_internal_file_drag_paths: Default::default(),
        }
    }
}

pub(in crate::native_app) struct MetadataAppState {
    pub(in crate::native_app) tag_draft: String,
    pub(in crate::native_app) tag_tokens: Vec<String>,
    pub(in crate::native_app) tag_input_mode: MetadataTagInputMode,
    pub(in crate::native_app) pending_tag_completion_query: Option<String>,
    pub(in crate::native_app) tag_completion_cycle: ui::CyclicListSelectionCycle,
    pub(in crate::native_app) tag_dictionary: BTreeMap<String, String>,
    pub(in crate::native_app) tag_library_open: bool,
    pub(in crate::native_app) tag_drag: Option<String>,
    pub(in crate::native_app) tag_drop_hover: Option<String>,
    pub(in crate::native_app) selected_tag: Option<String>,
    pub(in crate::native_app) collapsed_tag_categories: HashSet<String>,
    pub(in crate::native_app) tags_by_file: HashMap<String, Vec<String>>,
    pub(in crate::native_app) sample_name_view_mode: SampleNameViewMode,
}

impl MetadataAppState {
    pub(in crate::native_app) fn from_settings(settings: &AppSettingsCore) -> Self {
        Self {
            tag_draft: String::new(),
            tag_tokens: Vec::new(),
            tag_input_mode: Default::default(),
            pending_tag_completion_query: None,
            tag_completion_cycle: ui::CyclicListSelectionCycle::new(),
            tag_dictionary: settings.tag_dictionary.clone(),
            tag_library_open: false,
            tag_drag: None,
            tag_drop_hover: None,
            selected_tag: None,
            collapsed_tag_categories: Default::default(),
            tags_by_file: HashMap::new(),
            sample_name_view_mode: SampleNameViewMode::DiskFilename,
        }
    }
}

pub(in crate::native_app) struct WaveformLoadState {
    pub(in crate::native_app) progress: f32,
    pub(in crate::native_app) target_progress: f32,
    pub(in crate::native_app) label: Option<String>,
}

impl Default for WaveformLoadState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            target_progress: 0.0,
            label: None,
        }
    }
}

pub(in crate::native_app) struct WaveformCacheState {
    pub(in crate::native_app) entries: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::native_app) order: VecDeque<PathBuf>,
    pub(in crate::native_app) bytes: usize,
    pub(in crate::native_app) indicator_refresh_task: ui::LatestTask,
    pub(in crate::native_app) indicator_refresh_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheIndicatorRefreshResult>>>,
    pub(in crate::native_app) warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) warm_task: ui::LatestTask,
    pub(in crate::native_app) warm_results:
        Arc<Mutex<HashMap<ui::TaskTicket, WaveformCacheWarmResult>>>,
    pub(in crate::native_app) active_folder_warm_delay_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_folder_id: Option<String>,
    pub(in crate::native_app) active_folder_warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) cached_sample_paths: HashSet<String>,
}

impl Default for WaveformCacheState {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: Default::default(),
            bytes: 0,
            indicator_refresh_task: ui::LatestTask::new(),
            indicator_refresh_results: Default::default(),
            warm_pending: Default::default(),
            warm_task: ui::LatestTask::new(),
            warm_results: Default::default(),
            active_folder_warm_delay_task: ui::LatestTask::new(),
            active_folder_warm_task: ui::LatestTask::new(),
            active_folder_warm_cancel: None,
            active_folder_warm_folder_id: None,
            active_folder_warm_pending: Default::default(),
            cached_sample_paths: Default::default(),
        }
    }
}

pub(in crate::native_app) struct BackgroundTaskState {
    pub(in crate::native_app) worker_sender: Sender<GuiMessage>,
    pub(in crate::native_app) worker_receiver: Option<Receiver<GuiMessage>>,
    pub(in crate::native_app) next_task_id: u64,
    pub(in crate::native_app) deferred_sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) audio_open_task: ui::LatestTask,
    pub(in crate::native_app) audio_open_results:
        Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
    pub(in crate::native_app) startup_folder_verify_task: ui::LatestTask,
    pub(in crate::native_app) startup_folder_verify_results:
        Arc<Mutex<HashMap<ui::TaskTicket, FolderVerifyResult>>>,
    pub(in crate::native_app) normalization_progress: Option<NormalizationProgress>,
    pub(in crate::native_app) progress_tick: f32,
    pub(in crate::native_app) frame_cadence: ui::FrameCadenceMonitor,
}

impl BackgroundTaskState {
    pub(in crate::native_app) fn new(
        worker_sender: Sender<GuiMessage>,
        worker_receiver: Option<Receiver<GuiMessage>>,
    ) -> Self {
        Self {
            worker_sender,
            worker_receiver,
            next_task_id: 1,
            deferred_sample_load_task: ui::LatestTask::new(),
            sample_load_task: ui::LatestTask::new(),
            sample_load_cancel: None,
            audio_open_task: ui::LatestTask::new(),
            audio_open_results: Default::default(),
            startup_folder_verify_task: ui::LatestTask::new(),
            startup_folder_verify_results: Default::default(),
            normalization_progress: None,
            progress_tick: 0.0,
            frame_cadence: ui::FrameCadenceMonitor::new(),
        }
    }

    pub(in crate::native_app) fn next_task_id(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        task_id
    }

    #[cfg(test)]
    pub(in crate::native_app) fn for_tests() -> Self {
        Self::new(std::sync::mpsc::channel().0, None)
    }
}

pub(in crate::native_app) struct AudioAppState {
    pub(in crate::native_app) player: Option<AudioPlayer>,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) volume: f32,
    pub(in crate::native_app) volume_persist_deadline: Option<Instant>,
    pub(in crate::native_app) output_config: AudioOutputConfig,
    pub(in crate::native_app) output_resolved: Option<ResolvedOutput>,
    pub(in crate::native_app) hosts: Vec<AudioHostSummary>,
    pub(in crate::native_app) devices: Vec<AudioDeviceSummary>,
    pub(in crate::native_app) sample_rates: Vec<u32>,
    pub(in crate::native_app) settings_error: Option<String>,
    pub(in crate::native_app) current_playback_span: Option<(f32, f32)>,
    pub(in crate::native_app) pending_playback_start: Option<PendingPlaybackStart>,
    pub(in crate::native_app) pending_sample_playback: Option<PendingSamplePlayback>,
    pub(in crate::native_app) early_sample_playback_path: Option<String>,
}

impl AudioAppState {
    pub(in crate::native_app) fn from_settings(settings: &AppSettingsCore) -> Self {
        Self {
            player: None,
            loop_playback: false,
            volume: settings.volume.clamp(0.0, 1.0),
            volume_persist_deadline: None,
            output_config: settings.audio_output.clone(),
            output_resolved: None,
            hosts: Vec::new(),
            devices: Vec::new(),
            sample_rates: Vec::new(),
            settings_error: None,
            current_playback_span: None,
            pending_playback_start: None,
            pending_sample_playback: None,
            early_sample_playback_path: None,
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn for_tests() -> Self {
        Self::from_settings(&AppSettingsCore::default())
    }
}
