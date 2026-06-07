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
use crate::native_app::library_browser::context_menu_target::BrowserContextMenu;
use crate::native_app::library_browser::folder_browser::{
    FolderBrowserState, FolderScanProgress, FolderVerifyResult,
};
use crate::native_app::library_browser::source_watcher::GuiSourceWatcherHandle;
use crate::native_app::metadata::MetadataTagInputMode;
use crate::native_app::transaction_history::TransactionHistory;
use crate::native_app::waveform::WaveformState;

#[cfg(test)]
pub(in crate::native_app) const DEFAULT_VOLUME: f32 = 1.0;

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
