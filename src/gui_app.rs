//! Default Wavecrate GUI application built on Radiant's current public API.

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
    available_devices, available_hosts, supported_sample_rates,
};
use wavecrate::logging;
use wavecrate::sample_sources::SampleCollection;
#[cfg(test)]
use wavecrate::sample_sources::config::AppConfig;
use wavecrate::sample_sources::config::AppSettingsCore;

mod audio_engine;
mod audio_settings;
mod context_menu;
mod context_menu_actions;
mod drag_drop_actions;
mod file_actions;
mod folder_browser;
mod folder_browser_actions;
mod folder_browser_rename_actions;
mod folder_scan_actions;
mod launch;
mod layout;
mod lifecycle;
mod message_dispatch;
mod metadata_tags;
mod playback;
mod sample_browser_view;
mod sample_collections;
mod sample_load_actions;
mod sample_ratings;
mod selected_file_actions;
mod shortcuts;
mod status_bar;
mod toolbar;
mod waveform;
mod waveform_panel;
#[cfg(test)]
use audio_settings::audio_settings_popover;
use audio_settings::format_sample_rate_label;
#[cfg(test)]
use audio_settings::top_status_bar;
use context_menu::BrowserContextMenu;
#[cfg(test)]
use context_menu::BrowserContextTargetKind;
#[cfg(test)]
use file_actions::format_copy_path;
#[cfg(test)]
use file_actions::normalize_wav_file_in_place;
use file_actions::sample_path_label;
use folder_browser::{
    FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanResult,
};
use launch::emit_gui_action;
pub(crate) use launch::run;
#[cfg(test)]
use launch::{DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, debug_layout_requested};
use layout::view;
use metadata_tags::{MetadataTagInputMode, MetadataTagPersistResult};
#[cfg(test)]
use sample_browser_view::sample_browser;
use sample_load_actions::{NormalizedWaveformReload, WaveformPlaybackResume};
use shortcuts::default_gui_shortcut_resolution;
#[cfg(test)]
use toolbar::{
    TOOLBAR_FOCUS_LOADED_ID, TOOLBAR_STOP_ID, ToolbarIcon, toolbar_icon_button, toolbar_icon_svg,
};
use waveform::{WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState};

const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;
const SAMPLE_BROWSER_LIST_ID: u64 = 30_000;
const SAMPLE_BROWSER_ROW_HEIGHT: f32 = 22.0;
const SAMPLE_BROWSER_EDGE_CONTEXT_ROWS: usize = 2;
const SAMPLE_BROWSER_OVERSCAN_ROWS: usize = 4;
const SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS: usize = 128;
#[cfg(test)]
const DEFAULT_VOLUME: f32 = 1.0;
const VOLUME_SLIDER_ID: u64 = 31_000;
const VOLUME_SLIDER_WIDTH: f32 = 92.0;
const VOLUME_SLIDER_HEIGHT: f32 = 14.0;
const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);
const UI_FRAME_SPIKE_WARN: Duration = Duration::from_millis(34);
const UI_FRAME_SPIKE_ERROR: Duration = Duration::from_millis(100);
const UI_FRAME_PERIODIC_LOG_EVERY: u64 = 120;
const AUDIO_ENGINE_PILL_ID: u64 = 31_100;
const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 360.0;
const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 344.0;
const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
const DRAG_PREVIEW_HEIGHT: f32 = 20.0;
const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;
const WAVEFORM_SIGNAL_WIDGET_ID: u64 = 11;
const WAVEFORM_WIDGET_ID: u64 = 12;
const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration = Duration::from_millis(120);

#[derive(Clone, Debug, PartialEq)]
enum GuiMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    FolderScanProgress(FolderScanProgress),
    FolderScanDiscoveryBatch(FolderScanDiscoveryBatch),
    FolderScanFinished(FolderScanResult),
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
    SampleLoadProgress(ui::TaskTicket, f32),
    SampleLoadFinished(ui::TaskCompletion<SampleLoadResult>),
    AudioPlayerOpenFinished(ui::TaskTicket),
    PlaySelectedSample,
    StopPlayback,
    ToggleLoopPlayback,
    SetVolume(f32),
    ToggleAudioSettings,
    CloseAudioSettings,
    ToggleAudioBackendDropdown,
    ToggleAudioOutputDropdown,
    ToggleAudioSampleRateDropdown,
    CloseAudioSettingsDropdowns,
    SetAudioOutputHost(Option<String>),
    SetAudioOutputDevice(Option<String>),
    SetAudioOutputSampleRate(Option<u32>),
    MetadataTagInput(radiant::widgets::TextInputMessage),
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
    FocusLoadedFile,
    AdjustSelectedRating(i8),
    AssignSelectedCollection(SampleCollection),
    NormalizeSelectedSamples,
    CopySelectedFiles,
    CopyContextPath,
    OpenContextTarget,
    RemoveContextSource,
    CloseContextMenu,
    ToggleJobDetails,
    CloseJobDetails,
    Noop,
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
    DropWaveformSelectionOnSampleList,
    Waveform(WaveformInteraction),
    NativeFileDrop(NativeFileDrop),
    Frame,
}

#[derive(Clone, Debug)]
struct SampleLoadResult {
    path: String,
    result: Result<WaveformState, String>,
    autoplay: bool,
}

#[derive(Clone, Debug)]
struct WaveformCacheEntry {
    file: std::sync::Arc<waveform::WaveformFile>,
    signature: SampleFileSignature,
    byte_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SampleFileSignature {
    size_bytes: u64,
    modified_ns: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizationProgress {
    task_id: u64,
    label: String,
    completed: usize,
    total: usize,
    detail: String,
}

#[derive(Clone, Debug, PartialEq)]
struct NormalizationResult {
    task_id: u64,
    loaded_path: PathBuf,
    normalizing_loaded: bool,
    was_playing: bool,
    restart_ratio: f32,
    restart_span: Option<(f32, f32)>,
    normalized: Vec<PathBuf>,
    last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingPlaybackStart {
    start_ratio: f32,
    end_ratio: f32,
    loop_offset_ratio: Option<f32>,
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.result.as_ref().err() == other.result.as_ref().err()
    }
}

struct GuiAppState {
    folder_width: f32,
    folder_resize: Option<FolderResize>,
    folder_browser: FolderBrowserState,
    waveform: WaveformState,
    sample_status: String,
    worker_sender: Sender<GuiMessage>,
    worker_receiver: Option<Receiver<GuiMessage>>,
    next_task_id: u64,
    sample_load_task: ui::LatestTask,
    sample_load_cancel: Option<ui::CancellationToken>,
    audio_open_task: ui::LatestTask,
    audio_open_results: Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
    folder_progress: Option<FolderScanProgress>,
    normalization_progress: Option<NormalizationProgress>,
    progress_tick: f32,
    frame_index: u64,
    last_frame_at: Option<Instant>,
    max_frame_delta: Duration,
    waveform_loading_progress: f32,
    waveform_loading_target_progress: f32,
    audio_player: Option<AudioPlayer>,
    loop_playback: bool,
    volume: f32,
    volume_persist_deadline: Option<Instant>,
    audio_output_config: AudioOutputConfig,
    audio_output_resolved: Option<ResolvedOutput>,
    audio_hosts: Vec<AudioHostSummary>,
    audio_devices: Vec<AudioDeviceSummary>,
    audio_sample_rates: Vec<u32>,
    persisted_settings: AppSettingsCore,
    audio_settings_open: bool,
    audio_backend_dropdown_open: bool,
    audio_output_dropdown_open: bool,
    audio_sample_rate_dropdown_open: bool,
    job_details_open: bool,
    context_menu: Option<BrowserContextMenu>,
    waveform_loading_label: Option<String>,
    audio_settings_error: Option<String>,
    current_playback_span: Option<(f32, f32)>,
    pending_playback_start: Option<PendingPlaybackStart>,
    native_file_drop_hover: Option<NativeFileDropHover>,
    metadata_tag_draft: String,
    metadata_tag_tokens: Vec<String>,
    metadata_tag_input_mode: MetadataTagInputMode,
    metadata_tag_completion_prefix: Option<String>,
    metadata_tag_completion_index: usize,
    metadata_tag_dictionary: BTreeMap<String, String>,
    metadata_tag_library_open: bool,
    metadata_tag_drag: Option<String>,
    metadata_tag_drop_hover: Option<String>,
    selected_metadata_tag: Option<String>,
    collapsed_metadata_tag_categories: HashSet<String>,
    metadata_tags_by_file: HashMap<String, Vec<String>>,
    sample_name_view_mode: SampleNameViewMode,
    startup_auto_load_pending: bool,
    waveform_cache: HashMap<PathBuf, WaveformCacheEntry>,
    waveform_cache_order: VecDeque<PathBuf>,
    waveform_cache_bytes: usize,
    cached_sample_paths: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleNameViewMode {
    DiskFilename,
    MetadataLabel,
}

impl SampleNameViewMode {
    fn toggled(self) -> Self {
        match self {
            Self::DiskFilename => Self::MetadataLabel,
            Self::MetadataLabel => Self::DiskFilename,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeFileDropHover {
    path: PathBuf,
    supported: bool,
}

#[derive(Clone, Copy, Debug)]
struct FolderResize {
    start_x: f32,
    start_width: f32,
}

#[cfg(test)]
mod tests;
