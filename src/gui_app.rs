//! Default Wavecrate GUI application built on Radiant's current public API.

use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{
    NativeFileDrop, NativeFileDropPhase, NativeRunOptions, NativeTextOptions, PaintFillRect,
    PaintPrimitive,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, PointerModifiers, Widget, WidgetCommon, WidgetInput,
    WidgetOutput, WidgetSizing,
};
use rfd::FileDialog;
use std::{
    ffi::OsString,
    fs,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant, SystemTime},
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
    available_devices, available_hosts, supported_sample_rates,
};
use wavecrate::external_clipboard;
use wavecrate::gui_runtime::wavecrate_ui_font_path;
use wavecrate::logging::{self, ActionDebugEvent, emit_action_debug_event};
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

mod audio_engine;
mod audio_settings;
mod context_menu;
mod file_actions;
mod folder_browser;
mod playback;
mod sample_browser_view;
mod status_bar;
mod waveform;
#[cfg(test)]
use audio_settings::audio_settings_popover;
use audio_settings::{format_sample_rate_label, top_status_bar};
use context_menu::{BrowserContextMenu, BrowserContextTargetKind};
use file_actions::{
    format_copy_path, normalize_wav_file_in_place, open_folder_in_file_explorer,
    reveal_in_file_explorer, sample_path_label,
};
use folder_browser::{
    FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanRequest, FolderScanResult,
};
use sample_browser_view::sample_browser;
use waveform::{WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState};

const DEBUG_LAYOUT_ARG: &str = "--debug-layout";
const DEBUG_LAYOUT_SHORT_ARG: &str = "-debug-layout";
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
const AUDIO_ENGINE_PILL_ID: u64 = 31_100;
const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 360.0;
const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 344.0;
const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
const DRAG_PREVIEW_HEIGHT: f32 = 24.0;
const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;
const WAVEFORM_WIDGET_ID: u64 = 12;
const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration = Duration::from_millis(120);

#[derive(Clone, Debug, PartialEq)]
enum GuiMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    FolderScanProgress(FolderScanProgress),
    FolderScanDiscoveryBatch(FolderScanDiscoveryBatch),
    FolderScanFinished(FolderScanResult),
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
    ClearRebuildableCaches,
    NormalizeSelectedSamples,
    CopySelectedFiles,
    CopyContextPath,
    OpenContextTarget,
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
    Waveform(WaveformInteraction),
    NativeFileDrop(NativeFileDrop),
    Frame,
}

#[derive(Clone, Debug)]
struct SampleLoadResult {
    path: String,
    result: Result<WaveformState, String>,
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
    folder_progress: Option<FolderScanProgress>,
    progress_tick: f32,
    waveform_loading_progress: f32,
    waveform_loading_target_progress: f32,
    audio_player: Option<AudioPlayer>,
    loop_playback: bool,
    volume: f32,
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
    native_file_drop_hover: Option<NativeFileDropHover>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeFileDropHover {
    path: PathBuf,
    supported: bool,
}

impl GuiAppState {
    fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let config = wavecrate::sample_sources::config::load_or_default()
            .map_err(|err| format!("load app configuration: {err}"))?;
        let (worker_sender, worker_receiver) = mpsc::channel();
        let mut state = Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: FolderBrowserState::from_sample_sources(&config.sources),
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Select a sample to load"),
            worker_sender,
            worker_receiver: Some(worker_receiver),
            next_task_id: 1,
            sample_load_task: ui::LatestTask::new(),
            folder_progress: None,
            progress_tick: 0.0,
            waveform_loading_progress: 0.0,
            waveform_loading_target_progress: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: config.core.volume.clamp(0.0, 1.0),
            audio_output_config: config.core.audio_output.clone(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            persisted_settings: config.core,
            audio_settings_open: false,
            audio_backend_dropdown_open: false,
            audio_output_dropdown_open: false,
            audio_sample_rate_dropdown_open: false,
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            native_file_drop_hover: None,
        };
        state.refresh_audio_options();
        if let Err(error) = state.open_configured_audio_player() {
            state.audio_settings_error = Some(error);
        }
        emit_gui_action(
            "runtime.startup.load_default_state",
            Some("background"),
            Some("assets"),
            "success",
            started_at,
            None,
        );
        Ok(state)
    }

    fn persist_user_configuration(&mut self, action: &'static str, started_at: Instant) {
        if let Err(error) = self.save_user_configuration() {
            self.sample_status = format!("Settings not saved: {error}");
            emit_gui_action(
                action,
                Some("settings"),
                None,
                "persist_error",
                started_at,
                Some(&error),
            );
        }
    }

    fn save_user_configuration(&self) -> Result<(), String> {
        let mut core = self.persisted_settings.clone();
        core.audio_output = self.audio_output_config.clone();
        core.volume = self.volume;
        wavecrate::sample_sources::config::save(&AppConfig {
            sources: self.folder_browser.configured_sample_sources(),
            core,
        })
        .map_err(|err| err.to_string())
    }

    fn resize_folder_browser(&mut self, message: DragHandleMessage) {
        let started_at = Instant::now();
        let outcome = match message {
            DragHandleMessage::Started { .. } => "started",
            DragHandleMessage::Moved { .. } => "moved",
            DragHandleMessage::Ended { .. } => "ended",
        };
        match message {
            DragHandleMessage::Started { position } => {
                self.folder_resize = Some(FolderResize {
                    start_x: position.x,
                    start_width: self.folder_width,
                });
            }
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
                let resize = self.folder_resize.unwrap_or(FolderResize {
                    start_x: position.x,
                    start_width: self.folder_width,
                });
                self.folder_width = (resize.start_width + position.x - resize.start_x)
                    .clamp(MIN_FOLDER_WIDTH, MAX_FOLDER_WIDTH);
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.folder_resize = None;
                }
            }
        }
        emit_gui_action(
            "layout.resize_folder_browser",
            Some("folder_browser"),
            None,
            outcome,
            started_at,
            None,
        );
    }

    fn apply_folder_browser_message(
        &mut self,
        message: FolderBrowserMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            FolderBrowserMessage::AddSource => self.add_source_from_dialog(context),
            FolderBrowserMessage::SelectSource(id) => {
                self.context_menu = None;
                self.select_source(id, context);
            }
            FolderBrowserMessage::OpenSourceContextMenu(source_id, position) => {
                self.open_source_context_menu(source_id, position);
            }
            FolderBrowserMessage::BeginRenameSelected => self.begin_folder_browser_rename(context),
            FolderBrowserMessage::BeginCreateSubfolder => {
                self.begin_folder_browser_subfolder_creation(context);
            }
            FolderBrowserMessage::RenameInput(message) => {
                self.apply_folder_browser_rename_input(message);
            }
            FolderBrowserMessage::DropOnFolder(folder_id) => {
                self.context_menu = None;
                self.drop_browser_drag_on_folder(folder_id, context);
            }
            FolderBrowserMessage::OpenFolderContextMenu(folder_id, position) => {
                self.open_folder_context_menu(folder_id, position);
            }
            FolderBrowserMessage::DragFolder(folder_id, drag) => {
                self.context_menu = None;
                self.drag_folder(folder_id, drag, context);
            }
            message => self.folder_browser.apply_message(message),
        }
    }

    fn begin_folder_browser_rename(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        let target = self.folder_browser.selected_rename_target();
        if logging::debug_logging_enabled() {
            tracing::debug!(
                target: logging::ACTION_EVENT_TARGET,
                event = "action_detail",
                action = "folder_browser.rename.begin",
                pane = "folder_browser",
                target_kind = target.kind,
                target_label = target.label,
                is_source_root = target.is_source_root,
                "Folder browser rename requested"
            );
        }
        let renaming_file = self.folder_browser.selected_file_id().is_some();
        match self.folder_browser.begin_rename_selected() {
            Ok(Some(input_id)) => {
                self.sample_status = if renaming_file {
                    String::from("Renaming selected file")
                } else {
                    String::from("Renaming selected folder")
                };
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    Some(target.kind),
                    "success",
                    started_at,
                    None,
                );
            }
            Ok(None) => {
                self.sample_status = String::from("Select a folder to rename");
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some("nothing_selected"),
                );
            }
            Err(error) => {
                self.sample_status = error;
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    None,
                    "error",
                    started_at,
                    Some("rename_begin_failed"),
                );
            }
        }
    }

    fn begin_folder_browser_subfolder_creation(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.folder_browser.begin_create_subfolder() {
            Ok(Some(input_id)) => {
                self.sample_status = String::from("Creating new folder");
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    Some("folder"),
                    "success",
                    started_at,
                    None,
                );
            }
            Ok(None) => {
                self.sample_status = String::from("Select a folder to add a subfolder");
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some("nothing_selected"),
                );
            }
            Err(error) => {
                self.sample_status = error;
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    None,
                    "error",
                    started_at,
                    Some("create_begin_failed"),
                );
            }
        }
    }

    fn apply_folder_browser_rename_input(&mut self, message: radiant::widgets::TextInputMessage) {
        let started_at = Instant::now();
        let input_action = rename_input_action(&message);
        if let Some(status) = self.folder_browser.apply_rename_input(message) {
            self.sample_status = status;
        }
        if let Some(action) = input_action {
            emit_gui_action(
                action,
                Some("folder_browser"),
                None,
                "applied",
                started_at,
                None,
            );
        }
    }

    fn navigate_browser(
        &mut self,
        delta: i32,
        extend: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let direction = if delta < 0 { "previous" } else { "next" };
        let Some(path) = self.folder_browser.navigate_vertical(delta, extend) else {
            emit_gui_action(
                "folder_browser.navigate",
                Some("browser"),
                Some(direction),
                "edge",
                started_at,
                None,
            );
            return;
        };

        if let Some(index) = self.folder_browser.selected_audio_file_index() {
            context.scroll_fixed_row_into_view(
                SAMPLE_BROWSER_LIST_ID,
                index,
                SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                delta,
            );
        }
        emit_gui_action(
            "folder_browser.navigate",
            Some("browser"),
            Some(direction),
            "selected",
            started_at,
            None,
        );
        self.select_sample(path, context);
    }

    fn advance_frame(&mut self) {
        self.waveform.apply_interaction(WaveformInteraction::Frame);
        self.refresh_playback_progress();
        if self.folder_progress.is_some() {
            self.progress_tick = (self.progress_tick + 0.035) % 1.0;
        }
        if self.waveform_loading_label.is_some() {
            let remaining = self.waveform_loading_target_progress - self.waveform_loading_progress;
            if remaining > 0.0 {
                self.waveform_loading_progress += remaining.min(0.03);
            }
        }
    }

    fn apply_message(&mut self, message: GuiMessage, context: &mut ui::UpdateContext<GuiMessage>) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::FolderBrowser(message) => {
                self.apply_folder_browser_message(message, context);
            }
            GuiMessage::FolderScanProgress(progress) => {
                let started_at = Instant::now();
                if self
                    .folder_browser
                    .scan_is_active(&progress.source_id, progress.task_id)
                {
                    let phase = progress.phase.clone();
                    self.folder_progress = Some(progress);
                    emit_gui_action(
                        "folder_browser.scan.progress",
                        Some("folder_browser"),
                        Some(&phase),
                        "active",
                        started_at,
                        None,
                    );
                }
            }
            GuiMessage::FolderScanDiscoveryBatch(batch) => {
                let started_at = Instant::now();
                let count = batch.events.len();
                self.folder_browser.apply_scan_discovered_batch(batch);
                if logging::debug_logging_enabled() {
                    tracing::debug!(
                        target: logging::ACTION_EVENT_TARGET,
                        event = "action_detail",
                        action = "folder_browser.scan.discovery_batch",
                        pane = "folder_browser",
                        item_count = count,
                        "Folder browser scan discovery batch applied"
                    );
                }
                emit_gui_action(
                    "folder_browser.scan.discovery_batch",
                    Some("folder_browser"),
                    None,
                    "applied",
                    started_at,
                    None,
                );
            }
            GuiMessage::FolderScanFinished(result) => self.finish_folder_scan(result),
            GuiMessage::SelectSampleWithModifiers { path, modifiers } => {
                self.context_menu = None;
                self.select_sample_with_modifiers(path, modifiers, context);
            }
            GuiMessage::OpenSampleContextMenu { path, position } => {
                self.open_sample_context_menu(path, position);
            }
            GuiMessage::DragSampleFile { path, drag } => {
                self.context_menu = None;
                self.drag_sample_file(path, drag, context);
            }
            GuiMessage::ExternalDragCompleted(result) => {
                self.external_drag_completed(result, context)
            }
            GuiMessage::SampleLoadProgress(ticket, progress) => {
                if self.sample_load_task.is_active(ticket) {
                    self.waveform_loading_target_progress = progress.clamp(0.0, 0.995);
                }
            }
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result),
            GuiMessage::PlaySelectedSample => self.play_selected_sample(context),
            GuiMessage::StopPlayback => self.stop_playback(),
            GuiMessage::ToggleLoopPlayback => self.toggle_loop_playback(),
            GuiMessage::SetVolume(volume) => self.set_volume(volume),
            GuiMessage::ToggleAudioSettings => self.toggle_audio_settings(),
            GuiMessage::CloseAudioSettings => {
                self.close_audio_settings_window();
            }
            GuiMessage::ToggleAudioBackendDropdown => {
                self.audio_backend_dropdown_open = !self.audio_backend_dropdown_open;
                self.audio_output_dropdown_open = false;
                self.audio_sample_rate_dropdown_open = false;
            }
            GuiMessage::ToggleAudioOutputDropdown => {
                self.audio_output_dropdown_open = !self.audio_output_dropdown_open;
                self.audio_backend_dropdown_open = false;
                self.audio_sample_rate_dropdown_open = false;
            }
            GuiMessage::ToggleAudioSampleRateDropdown => {
                self.audio_sample_rate_dropdown_open = !self.audio_sample_rate_dropdown_open;
                self.audio_backend_dropdown_open = false;
                self.audio_output_dropdown_open = false;
            }
            GuiMessage::CloseAudioSettingsDropdowns => {
                self.close_audio_settings_dropdowns();
            }
            GuiMessage::SetAudioOutputHost(host) => self.set_audio_output_host(host),
            GuiMessage::SetAudioOutputDevice(device) => self.set_audio_output_device(device),
            GuiMessage::SetAudioOutputSampleRate(sample_rate) => {
                self.set_audio_output_sample_rate(sample_rate);
            }
            GuiMessage::ClearRebuildableCaches => self.clear_rebuildable_caches(),
            GuiMessage::NormalizeSelectedSamples => self.normalize_selected_samples(),
            GuiMessage::CopySelectedFiles => self.copy_selected_files(),
            GuiMessage::CopyContextPath => self.copy_context_path(),
            GuiMessage::OpenContextTarget => self.open_context_target(),
            GuiMessage::CloseContextMenu => {
                self.context_menu = None;
            }
            GuiMessage::ToggleJobDetails => {
                self.job_details_open = self.folder_progress.is_some() && !self.job_details_open;
            }
            GuiMessage::CloseJobDetails => {
                self.job_details_open = false;
            }
            GuiMessage::Noop => {}
            GuiMessage::FocusRenameInput(input_id) => {
                let started_at = Instant::now();
                context.focus(input_id);
                emit_gui_action(
                    "folder_browser.rename.focus_input",
                    Some("folder_browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(),
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(),
            GuiMessage::NavigateBrowser { delta, extend } => {
                self.navigate_browser(delta, extend, context);
            }
            GuiMessage::SelectAllSamples => {
                let started_at = Instant::now();
                let count = self.folder_browser.select_all_audio_files();
                self.sample_status = format!(
                    "Selected {count} sample{}",
                    if count == 1 { "" } else { "s" }
                );
                emit_gui_action(
                    "browser.select_all_samples",
                    Some("browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            GuiMessage::CollapseSelectedFolder => {
                let started_at = Instant::now();
                self.folder_browser.collapse_selected_folder();
                emit_gui_action(
                    "folder_browser.collapse_selected",
                    Some("folder_browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            GuiMessage::ExpandSelectedFolder => {
                let started_at = Instant::now();
                self.folder_browser.expand_selected_folder();
                emit_gui_action(
                    "folder_browser.expand_selected",
                    Some("folder_browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            GuiMessage::Waveform(message) => {
                let started_at = Instant::now();
                let action = waveform_interaction_action(&message);
                let active_drag = self.waveform.active_drag_kind();
                self.waveform.apply_interaction(message);
                self.sync_edit_fade_audio_state();
                if waveform_interaction_finishes_play_selection_edit(&message, active_drag) {
                    self.retarget_loop_playback_to_play_selection();
                }
                if let Some(action) = action {
                    emit_gui_action(action, Some("waveform"), None, "applied", started_at, None);
                }
                if let Some(start_ratio) = self.waveform.take_pending_playback_start() {
                    self.play_waveform_from_ratio(start_ratio);
                }
            }
            GuiMessage::NativeFileDrop(drop) => self.apply_native_file_drop(drop, context),
            GuiMessage::Frame => {
                self.advance_frame();
            }
        }
    }

    fn worker_subscription(&mut self) -> ui::Subscription<GuiMessage> {
        self.worker_receiver
            .take()
            .map(|receiver| ui::Subscription::worker("gui-workers", receiver))
            .unwrap_or_else(ui::Subscription::none)
    }

    fn next_folder_task_id(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        task_id
    }

    fn add_source_from_dialog(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        let Some(path) = FileDialog::new().set_title("Add source").pick_folder() else {
            emit_gui_action(
                "folder_browser.add_source_dialog",
                Some("folder_browser"),
                None,
                "cancelled",
                started_at,
                None,
            );
            return;
        };
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_add_source_path(path, task_id) {
            let label = request.label.clone();
            emit_gui_action(
                "folder_browser.add_source_dialog",
                Some("folder_browser"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            emit_gui_action(
                "folder_browser.add_source_dialog",
                Some("folder_browser"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_queued"),
            );
        }
    }

    fn select_source(&mut self, id: String, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_select_source(id, task_id) {
            let label = request.label.clone();
            emit_gui_action(
                "folder_browser.select_source",
                Some("folder_browser"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            emit_gui_action(
                "folder_browser.select_source",
                Some("folder_browser"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_found"),
            );
        }
    }

    fn delete_selected_item(&mut self) {
        if self.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files();
        } else {
            self.delete_selected_folder();
        }
    }

    fn delete_selected_folder(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        match self.folder_browser.delete_selected_folder() {
            Ok(status) => {
                self.sample_status = status;
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn delete_selected_files(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_file_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        let loaded_path = self.waveform.path();
        let deleting_loaded_sample = target.paths.iter().any(|path| path == &loaded_path);

        match self.folder_browser.delete_selected_files() {
            Ok(status) => {
                if deleting_loaded_sample {
                    if let Some(player) = self.audio_player.as_mut() {
                        player.stop();
                    }
                    self.waveform = WaveformState::empty();
                    self.current_playback_span = None;
                }
                self.sample_status = status;
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn extract_playmarked_range(&mut self) {
        let started_at = Instant::now();
        match self.waveform.extract_play_selection_to_sibling() {
            Ok(path) => {
                let label = sample_path_label(&path);
                self.waveform.flash_play_selection();
                self.folder_browser.refresh_file_path(&path);
                self.sample_status = format!("Extracted {label}");
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    Some(&label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn normalize_selected_samples(&mut self) {
        let started_at = Instant::now();
        let paths = self.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.sample_status = String::from("Select a sample to normalize");
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }

        let loaded_path = self.waveform.path();
        let normalizing_loaded = paths.iter().any(|path| path == &loaded_path);
        let was_playing = self.waveform.is_playing() && normalizing_loaded;
        let restart_ratio = self
            .audio_player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or(self.waveform.playhead_ratio())
            .unwrap_or(0.0);
        let restart_span = self.current_playback_span;
        if was_playing {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }

        let mut normalized = Vec::new();
        let mut last_error = None;
        for path in &paths {
            match normalize_wav_file_in_place(path) {
                Ok(()) => {
                    self.folder_browser.refresh_file_path(path);
                    normalized.push(path.clone());
                }
                Err(error) => {
                    last_error = Some(format!("{}: {error}", sample_path_label(path)));
                }
            }
        }

        if normalizing_loaded && normalized.iter().any(|path| path == &loaded_path) {
            if let Err(error) = self.reload_normalized_waveform(
                &loaded_path,
                was_playing,
                restart_ratio,
                restart_span,
            ) {
                last_error = Some(error);
            }
        }

        if let Some(error) = last_error {
            self.sample_status = format!(
                "Normalized {} sample{} | {error}",
                normalized.len(),
                if normalized.len() == 1 { "" } else { "s" }
            );
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "partial_or_error",
                started_at,
                Some(&error),
            );
            return;
        }

        self.sample_status = match normalized.as_slice() {
            [] => String::from("No selected samples were normalized"),
            [path] => format!("Normalized {}", sample_path_label(path)),
            _ => format!("Normalized {} samples", normalized.len()),
        };
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    fn reload_normalized_waveform(
        &mut self,
        path: &Path,
        resume_playback: bool,
        restart_ratio: f32,
        restart_span: Option<(f32, f32)>,
    ) -> Result<(), String> {
        self.waveform = WaveformState::load_path(path.to_path_buf())?;
        self.folder_browser.select_file(path.display().to_string());
        if resume_playback {
            let (_, previous_end) = restart_span.unwrap_or((0.0, 1.0));
            let start = restart_ratio.clamp(0.0, 1.0);
            let end = previous_end.max(start).clamp(start, 1.0);
            self.start_playback_current_span(start, end)?;
        }
        Ok(())
    }

    fn launch_folder_scan(
        &mut self,
        request: FolderScanRequest,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let label = request.label.clone();
        let root = request.root.display().to_string();
        self.folder_progress = Some(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Queued"),
            completed: 0,
            total: 0,
            detail: request.root.display().to_string(),
        });
        self.sample_status = format!("Scanning source {}", request.label);
        tracing::info!(
            source = label,
            root = root,
            task_id = request.task_id,
            "default gui: folder scan queued"
        );
        emit_gui_action(
            "folder_browser.scan.queue",
            Some("folder_browser"),
            Some(&label),
            "queued",
            started_at,
            None,
        );
        let sender = self.worker_sender.clone();
        context.spawn(
            "gui-folder-scan",
            move || {
                let discovery_sender = sender.clone();
                let mut pending_discoveries = Vec::with_capacity(64);
                let task_id = request.task_id;
                let source_id = request.source_id.clone();
                let result = folder_browser::scan_source_with_progress(
                    request,
                    |progress| {
                        let _ = sender.send(GuiMessage::FolderScanProgress(progress));
                    },
                    |event| {
                        pending_discoveries.push(event);
                        if pending_discoveries.len() >= 64 {
                            let events = std::mem::take(&mut pending_discoveries);
                            let _ = discovery_sender.send(GuiMessage::FolderScanDiscoveryBatch(
                                FolderScanDiscoveryBatch {
                                    task_id,
                                    source_id: source_id.clone(),
                                    events,
                                },
                            ));
                        }
                    },
                );
                if !pending_discoveries.is_empty() {
                    let events = std::mem::take(&mut pending_discoveries);
                    let _ = discovery_sender.send(GuiMessage::FolderScanDiscoveryBatch(
                        FolderScanDiscoveryBatch {
                            task_id,
                            source_id,
                            events,
                        },
                    ));
                }
                result
            },
            GuiMessage::FolderScanFinished,
        );
    }

    fn finish_folder_scan(&mut self, result: FolderScanResult) {
        let started_at = Instant::now();
        let label = result.label.clone();
        let file_count = result.file_count;
        let folder_count = result.folder_count;
        if self.folder_browser.apply_scan_finished(result) {
            self.folder_progress = None;
            self.job_details_open = false;
            self.progress_tick = 0.0;
            self.sample_status =
                format!("Loaded source {label}: {file_count} files in {folder_count} folders");
            tracing::info!(
                source = label,
                file_count,
                folder_count,
                "default gui: folder scan finished"
            );
            emit_gui_action(
                "folder_browser.scan.finish",
                Some("folder_browser"),
                Some(&label),
                "success",
                started_at,
                None,
            );
            self.persist_user_configuration("folder_browser.sources.persist", started_at);
        } else {
            emit_gui_action(
                "folder_browser.scan.finish",
                Some("folder_browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
        }
    }

    fn select_sample(&mut self, path: String, context: &mut ui::UpdateContext<GuiMessage>) {
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        self.load_sample(path, context);
    }

    fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        self.load_sample(path, context);
    }

    fn drag_sample_file(
        &mut self,
        path: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match drag {
            DragHandleMessage::Started { position } => {
                self.folder_browser.begin_file_drag(path, position);
                self.arm_browser_drag(context);
            }
            DragHandleMessage::Moved { position } => {
                self.folder_browser.update_drag_pointer(position);
            }
            DragHandleMessage::Ended { .. } => {
                self.folder_browser.clear_drag();
                context.end_drag();
                context.end_external_drag();
            }
        }
    }

    fn drag_folder(
        &mut self,
        folder_id: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started = matches!(drag, DragHandleMessage::Started { .. });
        let ended = matches!(drag, DragHandleMessage::Ended { .. });
        if ended {
            if let Some(target_folder_id) = self.folder_browser.hovered_drop_target_folder_id() {
                self.drop_browser_drag_on_folder(target_folder_id, context);
            } else {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                context.end_drag();
                context.end_external_drag();
            }
            return;
        }
        self.folder_browser
            .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
        if started {
            self.arm_browser_drag(context);
        }
    }

    fn arm_browser_drag(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        if let Some(preview) = self.folder_browser.drag_preview() {
            let width = folder_drag_preview_width(&preview.label);
            context.begin_drag(ui::DragRequest::new(
                ui::DragPreview::sized(preview.label, Vector2::new(width, DRAG_PREVIEW_HEIGHT)),
                preview.pointer,
            ));
        }
        let Some(request) = self.folder_browser.external_drag_request() else {
            return;
        };
        context.begin_external_drag(request, GuiMessage::ExternalDragCompleted);
    }

    fn copy_selected_files(&mut self) {
        let started_at = Instant::now();
        let paths = self.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.sample_status = String::from("Select files before copying");
            emit_gui_action(
                "browser.copy_selected_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("no selection"),
            );
            return;
        }

        match external_clipboard::copy_file_paths(&paths) {
            Ok(()) => {
                self.sample_status = match paths.len() {
                    1 => String::from("Copied selected file"),
                    count => format!("Copied {count} selected files"),
                };
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy failed: {error}");
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn open_source_context_menu(&mut self, source_id: String, position: Point) {
        let started_at = Instant::now();
        let Some(path) = self.folder_browser.source_root_path(&source_id) else {
            self.sample_status = String::from("Source is unavailable");
            emit_gui_action(
                "browser.context_menu.source.open",
                Some("sources"),
                None,
                "error",
                started_at,
                Some("source unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Source, &path) {
            self.sample_status = String::from("Source folder is missing");
            emit_gui_action(
                "browser.context_menu.source.open",
                Some("sources"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("source folder missing"),
            );
            return;
        }
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Source,
            path,
            anchor: position,
            title,
        });
    }

    fn open_folder_context_menu(&mut self, folder_id: String, position: Point) {
        let started_at = Instant::now();
        self.folder_browser
            .apply_message(FolderBrowserMessage::ActivateFolder(folder_id.clone()));
        let Some(path) = self.folder_browser.folder_path(&folder_id) else {
            self.sample_status = String::from("Folder is unavailable");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                None,
                "error",
                started_at,
                Some("folder unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Folder, &path) {
            self.sample_status = String::from("Folder is missing");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("folder missing"),
            );
            return;
        }
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Folder,
            path,
            anchor: position,
            title,
        });
    }

    fn open_sample_context_menu(&mut self, path: String, position: Point) {
        let started_at = Instant::now();
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        let Some(path) = self.folder_browser.context_sample_path(&path) else {
            self.sample_status = String::from("Sample is unavailable");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                None,
                "error",
                started_at,
                Some("sample unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Sample, &path) {
            self.sample_status = String::from("Sample file is missing");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("sample missing"),
            );
            return;
        }
        let title = sample_path_label(&path);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Sample,
            path,
            anchor: position,
            title,
        });
    }

    fn copy_context_path(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.copy_path",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let path_text = format_copy_path(&menu.path);
        match external_clipboard::copy_text(&path_text) {
            Ok(()) => {
                self.sample_status = String::from("Copied path");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy path failed: {error}");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn apply_native_file_drop(
        &mut self,
        drop: NativeFileDrop,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let over_waveform = drop.target_widget == Some(WAVEFORM_WIDGET_ID);
        match drop.phase {
            NativeFileDropPhase::Hover => {
                let Some(path) = drop.path else {
                    self.native_file_drop_hover = None;
                    return;
                };
                if over_waveform {
                    self.native_file_drop_hover = Some(NativeFileDropHover {
                        supported: supported_waveform_drop_file(&path),
                        path,
                    });
                } else {
                    self.native_file_drop_hover = None;
                }
            }
            NativeFileDropPhase::Cancel => {
                self.native_file_drop_hover = None;
            }
            NativeFileDropPhase::Drop => {
                self.native_file_drop_hover = None;
                let Some(path) = drop.path else {
                    return;
                };
                if !over_waveform {
                    return;
                }
                self.drop_external_file_on_waveform(path, context);
            }
        }
    }

    fn drop_external_file_on_waveform(
        &mut self,
        path: PathBuf,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !supported_waveform_drop_file(&path) {
            self.sample_status = format!(
                "Unsupported waveform drop: {}",
                path.file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| path.display().to_string().into())
            );
            emit_gui_action(
                "waveform.external_file_drop",
                Some("waveform"),
                None,
                "unsupported",
                started_at,
                Some("unsupported file type"),
            );
            return;
        }
        match self.copy_external_file_to_selected_folder(&path) {
            Ok(copied) => {
                let copied_id = copied.display().to_string();
                self.folder_browser.refresh_file_path(&copied);
                self.folder_browser.select_file(copied_id.clone());
                self.load_sample(copied_id, context);
                emit_gui_action(
                    "waveform.external_file_drop",
                    Some("waveform"),
                    None,
                    "copied",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("External drop failed: {error}");
                emit_gui_action(
                    "waveform.external_file_drop",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn copy_external_file_to_selected_folder(&mut self, source: &Path) -> Result<PathBuf, String> {
        if !source.is_file() {
            return Err(format!("not a file: {}", source.display()));
        }
        let target_folder = self
            .folder_browser
            .selected_folder_path()
            .ok_or_else(|| String::from("no selected folder"))?;
        fs::create_dir_all(&target_folder).map_err(|err| {
            format!(
                "failed to create target folder {}: {err}",
                target_folder.display()
            )
        })?;
        let file_name = source
            .file_name()
            .ok_or_else(|| String::from("dropped file has no file name"))?;
        let first_candidate = target_folder.join(file_name);
        let target = unique_copy_destination(&first_candidate);
        fs::copy(source, &target).map_err(|err| {
            format!(
                "failed to copy {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
        Ok(target)
    }

    fn open_context_target(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.open_explorer",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let result = match menu.kind {
            BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => {
                open_folder_in_file_explorer(&menu.path)
            }
            BrowserContextTargetKind::Sample => reveal_in_file_explorer(&menu.path),
        };
        match result {
            Ok(()) => {
                self.sample_status = match menu.kind {
                    BrowserContextTargetKind::Sample => String::from("Revealed sample"),
                    BrowserContextTargetKind::Source => String::from("Opened source folder"),
                    BrowserContextTargetKind::Folder => String::from("Opened folder"),
                };
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn external_drag_completed(
        &mut self,
        result: Result<ui::ExternalDragOutcome, String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        context.end_drag();
        self.folder_browser.clear_drag();
        self.sample_status = match result {
            Ok(outcome) if outcome.accepted() => match outcome.effect {
                ui::ExternalDragEffect::Copy => String::from("Dragged item externally"),
                ui::ExternalDragEffect::Move => String::from("Moved item externally"),
                ui::ExternalDragEffect::Link => String::from("Linked item externally"),
                ui::ExternalDragEffect::None => String::from("External drag cancelled"),
            },
            Ok(_) => String::from("External drag cancelled"),
            Err(error) => format!("External drag failed: {error}"),
        };
    }

    fn drop_browser_drag_on_folder(
        &mut self,
        folder_id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.end_drag();
        context.end_external_drag();
        match self.folder_browser.drop_drag_on_folder(&folder_id) {
            Ok(result) => {
                for (old_path, new_path) in &result.moved_paths {
                    self.waveform.rewrite_path_prefix(old_path, new_path);
                }
                if let Some(status) = result.status {
                    self.sample_status = status;
                }
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "unchanged"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                self.folder_browser.clear_drag();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn load_sample(&mut self, path: String, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        self.sample_status = format!("Loading {}", sample_path_label(path.as_str()));
        let label = sample_path_label(path.as_str());
        self.waveform_loading_label = Some(label.clone());
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
        let ticket = self.sample_load_task.begin();
        let sender = self.worker_sender.clone();
        context.spawn(
            "gui-sample-load",
            move || {
                let result =
                    WaveformState::load_path_with_progress(PathBuf::from(&path), |progress| {
                        let _ = sender.send(GuiMessage::SampleLoadProgress(ticket, progress));
                    });
                ui::TaskCompletion {
                    ticket,
                    output: SampleLoadResult { path, result },
                }
            },
            GuiMessage::SampleLoadFinished,
        );
    }

    fn finish_sample_load(&mut self, load: ui::TaskCompletion<SampleLoadResult>) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.sample_load_task.finish(ticket) {
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        match load.result {
            Ok(waveform) => {
                let file_name = waveform.file_name();
                self.waveform = waveform;
                match self.start_playback_current_span(0.0, 1.0) {
                    Ok(()) => {
                        self.sample_status = format!("Playing {file_name}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "playing",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        self.sample_status =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "loaded_playback_error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
            }
            Err(err) => {
                self.sample_status = format!("Could not load sample: {err}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }
}

fn waveform_interaction_finishes_play_selection_edit(
    interaction: &WaveformInteraction,
    active_drag: Option<WaveformActiveDragKind>,
) -> bool {
    if !matches!(interaction, WaveformInteraction::FinishSelection { .. }) {
        return false;
    }
    matches!(
        active_drag,
        Some(WaveformActiveDragKind::Selection(
            WaveformSelectionKind::Play
        )) | Some(WaveformActiveDragKind::SelectionResize(
            WaveformSelectionKind::Play,
            _
        )) | Some(WaveformActiveDragKind::SelectionMove(
            WaveformSelectionKind::Play
        ))
    )
}

#[derive(Clone, Copy, Debug)]
struct FolderResize {
    start_x: f32,
    start_width: f32,
}

/// Run the default Radiant GUI application shell.
pub(crate) fn run() -> Result<(), String> {
    logging::install_panic_hook();
    let args: Vec<OsString> = std::env::args_os().collect();
    let startup_started_at = Instant::now();

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested(&args) {
        enable_windows_console();
    }

    if let Err(err) = logging::init(args.iter().cloned()) {
        eprintln!("logging disabled: {err}");
    }

    log_default_gui_startup(&args);
    let state = GuiAppState::load_default()?;
    let options = NativeRunOptions {
        title: String::from("Wavecrate"),
        inner_size: Some([960.0, 540.0]),
        min_inner_size: Some([640.0, 360.0]),
        debug_layout: debug_layout_requested(args.iter().cloned()),
        text: NativeTextOptions {
            embedded_fonts: Vec::new(),
            font_paths: vec![wavecrate_ui_font_path()],
        },
        ..NativeRunOptions::default()
    };
    tracing::info!(
        debug_layout = options.debug_layout,
        "default gui: preparing Radiant application"
    );
    emit_gui_action(
        "runtime.startup.prepare_radiant_app",
        Some("background"),
        None,
        "running",
        startup_started_at,
        None,
    );

    let run_result = panic::catch_unwind(AssertUnwindSafe(|| {
        radiant::app(state)
            .options(options)
            .view(view)
            .animation(|state| {
                state.waveform.is_playing()
                    || state.waveform.play_selection_flash_active()
                    || state.folder_progress.is_some()
                    || state.waveform_loading_label.is_some()
            })
            .on_frame(|| GuiMessage::Frame)
            .subscriptions(GuiAppState::worker_subscription)
            .auxiliary_windows(audio_settings::auxiliary_windows)
            .on_scroll(|state, update, _context| {
                if update.node_id == SAMPLE_BROWSER_LIST_ID {
                    state.folder_browser.set_file_view_start_from_scroll_offset(
                        update.offset.y,
                        SAMPLE_BROWSER_ROW_HEIGHT,
                    );
                }
            })
            .on_native_file_drop(|_state, drop, context| {
                context.emit(GuiMessage::NativeFileDrop(drop));
            })
            .shortcuts(|state, _, press, _| default_gui_shortcut_resolution(state, press))
            .update_with(|state, message, context| {
                state.apply_message(message, context);
                context.request_repaint();
            })
            .run()
    }));

    match run_result {
        Ok(Ok(())) => {
            tracing::info!("default gui: Radiant runtime exited normally");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "success",
                startup_started_at,
                None,
            );
            Ok(())
        }
        Ok(Err(err)) => {
            tracing::error!(err = %err, "default gui: Radiant runtime exited with error");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "error",
                startup_started_at,
                Some(&err),
            );
            Err(err)
        }
        Err(payload) => {
            let message = panic_payload_to_string(payload);
            tracing::error!("default gui: panic captured while running: {message}");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "panic",
                startup_started_at,
                Some(&message),
            );
            Err(format!("startup panic: {message}"))
        }
    }
}

fn debug_layout_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .any(|arg| arg == DEBUG_LAYOUT_ARG || arg == DEBUG_LAYOUT_SHORT_ARG)
}

fn log_default_gui_startup(args: &[OsString]) {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("<unknown>"));
    tracing::info!(
        pid = process::id(),
        exe = exe,
        cwd = cwd,
        arg_count = args.len(),
        timestamp = ?SystemTime::now(),
        debug = cfg!(debug_assertions),
        "default gui startup: process metadata captured"
    );
    match wavecrate::app_dirs::resolve_persistence() {
        Ok(persistence) => {
            tracing::info!(
                persistence_mode = %persistence.mode,
                config_base = %persistence.config_base.display(),
                app_root = %persistence.app_root.display(),
                "default gui startup: persistence profile resolved"
            );
        }
        Err(err) => {
            tracing::error!(err = %err, "default gui startup: failed to resolve persistence profile");
        }
    }
}

fn emit_gui_action(
    action: &'static str,
    pane: Option<&'static str>,
    source: Option<&str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane,
        source,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}

fn rename_input_action(message: &radiant::widgets::TextInputMessage) -> Option<&'static str> {
    match message {
        radiant::widgets::TextInputMessage::Submitted { .. } => {
            Some("folder_browser.rename.submit")
        }
        _ => None,
    }
}

fn waveform_interaction_action(interaction: &WaveformInteraction) -> Option<&'static str> {
    match interaction {
        WaveformInteraction::Wheel { .. } => Some("waveform.zoom_wheel"),
        WaveformInteraction::ScrollTo { .. } => Some("waveform.scroll"),
        WaveformInteraction::BeginSelection { .. } => Some("waveform.selection.begin"),
        WaveformInteraction::BeginEditFade { .. } => Some("waveform.edit_fade.begin"),
        WaveformInteraction::ClearEditFadeSilence { .. } => {
            Some("waveform.edit_fade.clear_silence")
        }
        WaveformInteraction::BeginSelectionResize { .. } => Some("waveform.selection.resize_begin"),
        WaveformInteraction::BeginSelectionMove { .. } => Some("waveform.selection.move_begin"),
        WaveformInteraction::BeginPan { .. } => Some("waveform.pan_begin"),
        WaveformInteraction::FinishSelection { .. } => Some("waveform.selection.finish"),
        WaveformInteraction::UpdateSelection { .. } | WaveformInteraction::Frame => None,
    }
}

fn panic_payload_to_string(panic_payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic_payload.downcast_ref::<&str>() {
        return message.to_string();
    }
    if let Some(message) = panic_payload.downcast_ref::<String>() {
        return message.clone();
    }

    "<non-string panic payload>".to_string()
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn log_console_requested(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        arg == &OsString::from(logging::DEBUG_LOGGING_SHORT_ARG)
            || arg == &OsString::from(logging::DEBUG_LOGGING_ARG)
    })
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn enable_windows_console() {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };

    unsafe {
        let attached = AttachConsole(ATTACH_PARENT_PROCESS).is_ok();
        if !attached {
            let _ = AllocConsole();
        }

        let Ok(handle) = CreateFileW(
            windows::core::w!("CONOUT$"),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) else {
            return;
        };

        let handle = HANDLE(handle.0);
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
        let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
    }
}

fn view(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if state.job_details_open {
        if let Some(progress) = state.folder_progress.as_ref() {
            layers.push(status_bar::job_details_popover(progress));
        }
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(context_menu::overlay(menu));
    }
    if layers.len() > 1 {
        ui::stack(layers).fill()
    } else {
        layers.pop().expect("view should contain base content")
    }
}

fn folder_drag_preview_width(label: &str) -> f32 {
    (label.chars().count() as f32 * 7.0 + 118.0).clamp(150.0, DRAG_PREVIEW_MAX_WIDTH)
}

fn supported_waveform_drop_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn unique_copy_destination(first_candidate: &Path) -> PathBuf {
    if !first_candidate.exists() {
        return first_candidate.to_path_buf();
    }
    let parent = first_candidate.parent().unwrap_or_else(|| Path::new(""));
    let stem = first_candidate
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = first_candidate
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}

fn default_gui_shortcut_resolution(
    state: &GuiAppState,
    press: ui::KeyPress,
) -> ui::ShortcutResolution<GuiMessage> {
    if state.folder_browser.rename_active() {
        ui::ShortcutResolution::unhandled()
    } else if state.context_menu.is_some() {
        ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseContextMenu,
            )
            .resolve(press)
    } else if state.audio_settings_dropdown_open() {
        ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseAudioSettingsDropdowns,
            )
            .resolve(press)
    } else if state.job_details_open {
        ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseJobDetails,
            )
            .resolve(press)
    } else {
        let n_action = if state.folder_browser.selected_file_id().is_some() {
            GuiMessage::NormalizeSelectedSamples
        } else {
            GuiMessage::FolderBrowser(FolderBrowserMessage::BeginCreateSubfolder)
        };

        ui::ShortcutLayer::new()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::StopPlayback,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::F2),
                GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected),
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::Delete),
                GuiMessage::DeleteSelectedItem,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::Backspace),
                GuiMessage::DeleteSelectedItem,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::E),
                GuiMessage::ExtractPlaymarkedRange,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::L),
                GuiMessage::ToggleLoopPlayback,
            )
            .bind(ui::KeyPress::new(ui::KeyCode::N), n_action)
            .bind(
                ui::KeyPress::new(ui::KeyCode::Space),
                GuiMessage::PlaySelectedSample,
            )
            .bind(
                ui::KeyPress::with_command(ui::KeyCode::A),
                GuiMessage::SelectAllSamples,
            )
            .bind(
                ui::KeyPress::with_command(ui::KeyCode::C),
                GuiMessage::CopySelectedFiles,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::ArrowLeft),
                GuiMessage::CollapseSelectedFolder,
            )
            .bind(
                ui::KeyPress::new(ui::KeyCode::ArrowRight),
                GuiMessage::ExpandSelectedFolder,
            )
            .resolve_or_else(press, || match press.key {
                ui::KeyCode::ArrowUp => {
                    ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
                        delta: -1,
                        extend: press.shift,
                    })
                }
                ui::KeyCode::ArrowDown => {
                    ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
                        delta: 1,
                        extend: press.shift,
                    })
                }
                _ => ui::ShortcutResolution::unhandled(),
            })
    }
}

fn center_panel(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::row([folder_sidebar(state), folder_splitter(), main_area(state)])
        .padding(6.0)
        .fill()
}

fn folder_sidebar(state: &GuiAppState) -> ui::View<GuiMessage> {
    folder_browser::folder_browser_view(&state.folder_browser)
        .width(state.folder_width)
        .fill_height()
}

fn folder_splitter() -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill(),
        ui::drag_handle()
            .mapped(GuiMessage::ResizeFolder)
            .key("folder-browser-splitter-handle")
            .size(5.0, 32.0),
        ui::spacer().fill(),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .width(11.0)
    .fill_height()
    .padding(2.0)
    .spacing(4.0)
}

fn main_area(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        main_toolbar(state),
        waveform_panel(state),
        sample_browser(state),
    ])
    .padding(4.0)
    .fill()
}

fn main_toolbar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        ui::spacer().height(24.0).fill_width(),
        toolbar_icon_button(19, ToolbarIcon::Loop, true, state.loop_playback),
        toolbar_icon_button(20, ToolbarIcon::Play, true, state.waveform.is_playing()),
        toolbar_icon_button(21, ToolbarIcon::Stop, state.waveform.is_playing(), false),
    ])
    .padding_y(3.0)
    .style(ui::WidgetStyle::default())
    .spacing(4.0)
    .fill_width()
    .height(34.0)
}

fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    let Some(svg_icon) = ui::SvgIcon::from_svg(icon.svg()) else {
        return ui::button("")
            .message(toolbar_button_message(icon))
            .id(id)
            .size(28.0, 24.0);
    };
    ui::icon_button(svg_icon)
        .enabled(enabled)
        .active(active)
        .message(toolbar_button_message(icon))
        .id(id)
        .size(28.0, 24.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarIcon {
    Loop,
    Play,
    Stop,
}

impl ToolbarIcon {
    fn svg(self) -> &'static str {
        match self {
            Self::Loop => include_str!("gui_app/assets/icons/waveform_toolbar/loop.svg"),
            Self::Play => include_str!("gui_app/assets/icons/waveform_toolbar/play.svg"),
            Self::Stop => include_str!("gui_app/assets/icons/waveform_toolbar/stop.svg"),
        }
    }
}

fn toolbar_button_message(icon: ToolbarIcon) -> GuiMessage {
    match icon {
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}

fn waveform_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        waveform_panel_header(&state.waveform),
        ui::text(waveform_title(&state.waveform))
            .height(18.0)
            .fill_width()
            .truncate(),
        waveform_viewport_with_loading_state(state),
        waveform_scrollbar(&state.waveform),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(WAVEFORM_PANEL_HEIGHT)
}

fn waveform_panel_header(_waveform: &WaveformState) -> ui::View<GuiMessage> {
    ui::text("Waveform").height(18.0).fill_width()
}

fn waveform_viewport_with_loading_state(state: &GuiAppState) -> ui::View<GuiMessage> {
    let viewport = waveform::waveform_viewport_view(&state.waveform)
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT);
    let mut layers = vec![viewport];
    if let Some(hover) = state.native_file_drop_hover.as_ref() {
        layers.push(waveform_drop_hover_visual(hover.supported));
    }
    if state.waveform_loading_label.is_some() {
        layers.push(waveform_loading_visual(
            state.waveform_loading_label.as_deref().unwrap_or_default(),
            state.waveform_loading_progress,
        ));
        layers.push(
            ui::custom_widget_mapped(WaveformLoadingInputBlocker::new(), |message: GuiMessage| {
                message
            })
            .key("waveform-loading-input-blocker")
            .input_only()
            .fill_width()
            .height(WAVEFORM_VIEW_HEIGHT),
        );
    }
    if layers.len() == 1 {
        layers.pop().expect("viewport layer")
    } else {
        ui::stack(layers).fill_width().height(WAVEFORM_VIEW_HEIGHT)
    }
}

fn waveform_loading_visual(_label: &str, progress: f32) -> ui::View<GuiMessage> {
    ui::custom_widget(WaveformLoadingVisual::new(progress), |_| None)
        .key("waveform-loading-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

fn waveform_drop_hover_visual(supported: bool) -> ui::View<GuiMessage> {
    ui::custom_widget(WaveformDropHoverVisual::new(supported), |_| None)
        .key("waveform-drop-hover-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

#[derive(Clone, Debug)]
struct WaveformDropHoverVisual {
    common: WidgetCommon,
    supported: bool,
}

impl WaveformDropHoverVisual {
    fn new(supported: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, supported }
    }
}

impl Widget for WaveformDropHoverVisual {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let (r, g, b) = if self.supported {
            (74, 178, 116)
        } else {
            (214, 62, 62)
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 { r, g, b, a: 56 },
        }));
        let edge = 3.0_f32.min(bounds.height().max(1.0));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                bounds.min,
                Point::new(bounds.max.x, (bounds.min.y + edge).min(bounds.max.y)),
            ),
            color: Rgba8 { r, g, b, a: 210 },
        }));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x, (bounds.max.y - edge).max(bounds.min.y)),
                bounds.max,
            ),
            color: Rgba8 { r, g, b, a: 210 },
        }));
    }
}

#[derive(Clone, Debug)]
struct WaveformLoadingVisual {
    common: WidgetCommon,
    progress: f32,
}

impl WaveformLoadingVisual {
    fn new(progress: f32) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            progress: progress.clamp(0.0, 1.0),
        }
    }
}

impl Widget for WaveformLoadingVisual {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 {
                r: 22,
                g: 24,
                b: 25,
                a: 72,
            },
        }));

        let fill_width = bounds.width() * self.progress;
        if fill_width > 0.5 {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    bounds.min,
                    Point::new((bounds.min.x + fill_width).min(bounds.max.x), bounds.max.y),
                ),
                color: Rgba8 {
                    r: 174,
                    g: 178,
                    b: 181,
                    a: 118,
                },
            }));
        }
    }
}

#[derive(Clone, Debug)]
struct WaveformLoadingInputBlocker {
    common: WidgetCommon,
}

impl WaveformLoadingInputBlocker {
    fn new() -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for WaveformLoadingInputBlocker {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position }
            | WidgetInput::PointerPress { position, .. }
            | WidgetInput::PointerRelease { position, .. }
            | WidgetInput::PointerDrop { position, .. }
                if bounds.contains(position) =>
            {
                Some(WidgetOutput::typed(GuiMessage::Noop))
            }
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

fn waveform_title(waveform: &WaveformState) -> String {
    if !waveform.has_loaded_sample() {
        return String::from("No sample loaded");
    }
    format!(
        "{} | {} Hz | {} channel{} -> mono | {} frames",
        waveform.file_name(),
        waveform.sample_rate(),
        waveform.channels(),
        if waveform.channels() == 1 { "" } else { "s" },
        waveform.frames()
    )
}

fn waveform_scrollbar(waveform: &WaveformState) -> ui::View<GuiMessage> {
    if waveform.fully_zoomed_out() {
        return ui::text("").fill_width().height(0.0);
    }
    ui::scrollbar(ui::ScrollbarAxis::Horizontal)
        .viewport_fraction(waveform.visible_fraction())
        .offset_fraction(waveform.offset_fraction())
        .mapped(|message| match message {
            ui::ScrollbarMessage::OffsetChanged { offset_fraction } => {
                GuiMessage::Waveform(WaveformInteraction::ScrollTo { offset_fraction })
            }
        })
        .fill_width()
        .height(6.0)
}

#[cfg(test)]
mod tests {
    use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
    use super::{
        DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_FOLDER_WIDTH, GuiAppState,
        MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction, debug_layout_requested,
    };
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        prelude::{self as ui, IntoView},
        runtime::{NativeFileDrop, PaintPrimitive},
        widgets::{DragHandleMessage, PointerButton, PointerModifiers, Widget, WidgetInput},
    };
    use std::{ffi::OsString, fs, path::PathBuf, sync::mpsc};

    fn selected_asset_file_path(browser: &super::FolderBrowserState, name: &str) -> String {
        browser
            .selected_audio_files()
            .iter()
            .find(|file| file.name == name)
            .unwrap_or_else(|| panic!("expected bundled asset {name} to be visible"))
            .id
            .clone()
    }

    fn gui_state_for_span_tests() -> GuiAppState {
        GuiAppState {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: super::FolderBrowserState::load_default(),
            waveform: super::WaveformState::synthetic_for_tests(),
            sample_status: String::new(),
            worker_sender: mpsc::channel().0,
            worker_receiver: None,
            next_task_id: 1,
            sample_load_task: ui::LatestTask::new(),
            folder_progress: None,
            progress_tick: 0.0,
            waveform_loading_progress: 0.0,
            waveform_loading_target_progress: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            persisted_settings: super::AppSettingsCore::default(),
            audio_settings_open: false,
            audio_backend_dropdown_open: false,
            audio_output_dropdown_open: false,
            audio_sample_rate_dropdown_open: false,
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            native_file_drop_hover: None,
        }
    }

    #[test]
    fn canonical_debug_layout_arg_enables_default_gui_overlay() {
        assert!(debug_layout_requested([
            OsString::from("wavecrate"),
            OsString::from(DEBUG_LAYOUT_ARG),
        ]));
    }

    #[test]
    fn short_debug_layout_arg_enables_default_gui_overlay() {
        assert!(debug_layout_requested([
            OsString::from("wavecrate"),
            OsString::from(DEBUG_LAYOUT_SHORT_ARG),
        ]));
    }

    #[test]
    fn unrelated_args_leave_default_gui_overlay_disabled() {
        assert!(!debug_layout_requested([
            OsString::from("wavecrate"),
            OsString::from("--debug-log"),
        ]));
    }

    #[test]
    fn escape_shortcut_routes_to_stop_playback() {
        let state = GuiAppState::load_default().expect("default state loads");
        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(resolution.action, Some(super::GuiMessage::StopPlayback));
        assert!(resolution.handled);
    }

    #[test]
    fn escape_shortcut_is_shielded_while_renaming() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
        state.folder_browser.select_file(sample_path);
        state
            .folder_browser
            .begin_rename_selected()
            .expect("begin rename should not fail");

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(resolution, ui::ShortcutResolution::unhandled());
    }

    #[test]
    fn audio_settings_window_does_not_capture_main_escape_shortcut() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(resolution.action, Some(super::GuiMessage::StopPlayback));
        assert!(resolution.handled);
    }

    #[test]
    fn audio_settings_window_does_not_block_main_shortcuts() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::N));

        assert!(matches!(
            resolution.action,
            Some(super::GuiMessage::FolderBrowser(
                super::FolderBrowserMessage::BeginCreateSubfolder
            ))
        ));
        assert!(resolution.handled);
    }

    #[test]
    fn context_menu_escape_shortcut_closes_context_menu() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.context_menu = Some(super::BrowserContextMenu {
            kind: super::BrowserContextTargetKind::Sample,
            path: std::path::PathBuf::from("C:\\samples\\kick.wav"),
            anchor: Point::new(12.0, 24.0),
            title: String::from("kick.wav"),
        });

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(resolution.action, Some(super::GuiMessage::CloseContextMenu));
        assert!(resolution.handled);
    }

    #[test]
    fn audio_backend_dropdown_escape_shortcut_closes_dropdown() {
        let mut state = gui_state_for_span_tests();
        state.audio_backend_dropdown_open = true;

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(
            resolution.action,
            Some(super::GuiMessage::CloseAudioSettingsDropdowns)
        );
        assert!(resolution.handled);
    }

    #[test]
    fn format_copy_path_uses_forward_slashes_and_quotes_spaces() {
        assert_eq!(
            super::format_copy_path(std::path::Path::new("C:\\sample folder\\kick.wav")),
            "\"C:/sample folder/kick.wav\""
        );
        assert_eq!(
            super::format_copy_path(std::path::Path::new("C:\\samples\\kick.wav")),
            "C:/samples/kick.wav"
        );
    }

    #[test]
    fn context_menu_availability_requires_existing_target_kind() {
        let root = std::env::temp_dir().join(format!(
            "wavecrate-context-menu-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        let sample = root.join("kick.wav");
        std::fs::write(&sample, [0_u8; 8]).expect("write sample");

        assert!(super::context_menu::target_available(
            &super::BrowserContextTargetKind::Source,
            &root
        ));
        assert!(super::context_menu::target_available(
            &super::BrowserContextTargetKind::Folder,
            &root
        ));
        assert!(super::context_menu::target_available(
            &super::BrowserContextTargetKind::Sample,
            &sample
        ));
        assert!(!super::context_menu::target_available(
            &super::BrowserContextTargetKind::Sample,
            &root
        ));
        assert!(!super::context_menu::target_available(
            &super::BrowserContextTargetKind::Folder,
            &sample
        ));

        std::fs::remove_file(&sample).expect("remove sample");
        assert!(!super::context_menu::target_available(
            &super::BrowserContextTargetKind::Sample,
            &sample
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stale_context_menu_copy_path_refuses_missing_sample_file() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.context_menu = Some(super::BrowserContextMenu {
            kind: super::BrowserContextTargetKind::Sample,
            path: std::env::temp_dir().join("wavecrate-missing-context-sample.wav"),
            anchor: Point::new(12.0, 24.0),
            title: String::from("missing.wav"),
        });

        state.copy_context_path();

        assert_eq!(state.sample_status, "Sample file is missing");
        assert_eq!(state.context_menu, None);
    }

    #[test]
    fn copy_shortcut_routes_to_browser_file_handoff() {
        let state = GuiAppState::load_default().expect("default state loads");
        let resolution = super::default_gui_shortcut_resolution(
            &state,
            ui::KeyPress::with_command(ui::KeyCode::C),
        );

        assert_eq!(
            resolution.action,
            Some(super::GuiMessage::CopySelectedFiles)
        );
        assert!(resolution.handled);
    }

    #[test]
    fn backspace_shortcut_routes_to_delete_selected_item() {
        let state = GuiAppState::load_default().expect("default state loads");
        let resolution = super::default_gui_shortcut_resolution(
            &state,
            ui::KeyPress::new(ui::KeyCode::Backspace),
        );

        assert_eq!(
            resolution.action,
            Some(super::GuiMessage::DeleteSelectedItem)
        );
        assert!(resolution.handled);
    }

    #[test]
    fn loop_shortcut_routes_to_loop_toggle() {
        let state = GuiAppState::load_default().expect("default state loads");
        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::L));

        assert_eq!(
            resolution.action,
            Some(super::GuiMessage::ToggleLoopPlayback)
        );
        assert!(resolution.handled);
    }

    #[test]
    fn folder_browser_splitter_resizes_and_clamps_width() {
        let mut state = GuiAppState {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: super::FolderBrowserState::load_default(),
            waveform: super::WaveformState::synthetic_for_tests(),
            sample_status: String::new(),
            worker_sender: mpsc::channel().0,
            worker_receiver: None,
            next_task_id: 1,
            sample_load_task: ui::LatestTask::new(),
            folder_progress: None,
            progress_tick: 0.0,
            waveform_loading_progress: 0.0,
            waveform_loading_target_progress: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            persisted_settings: super::AppSettingsCore::default(),
            audio_settings_open: false,
            audio_backend_dropdown_open: false,
            audio_output_dropdown_open: false,
            audio_sample_rate_dropdown_open: false,
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            native_file_drop_hover: None,
        };
        state.resize_folder_browser(DragHandleMessage::Started {
            position: Point::new(100.0, 0.0),
        });
        state.resize_folder_browser(DragHandleMessage::Moved {
            position: Point::new(160.0, 0.0),
        });

        assert_eq!(state.folder_width, DEFAULT_FOLDER_WIDTH + 60.0);

        state.resize_folder_browser(DragHandleMessage::Moved {
            position: Point::new(900.0, 0.0),
        });
        assert_eq!(state.folder_width, MAX_FOLDER_WIDTH);

        state.resize_folder_browser(DragHandleMessage::Ended {
            position: Point::new(-900.0, 0.0),
        });
        assert_eq!(state.folder_width, MIN_FOLDER_WIDTH);
        assert!(state.folder_resize.is_none());
    }

    #[test]
    fn default_gui_starts_without_loading_a_sample() {
        let waveform = super::WaveformState::load_default().expect("default sample loads");
        assert!(!waveform.has_loaded_sample());
        assert_eq!(waveform.file_name(), "No sample loaded");
    }

    #[test]
    fn looped_waveform_click_resolves_to_full_sample_without_playmark() {
        let mut state = gui_state_for_span_tests();
        state.loop_playback = true;

        let span = state.resolve_playback_span(0.45, 1.0, None);

        assert_eq!(span.start_ratio, 0.0);
        assert_eq!(span.end_ratio, 1.0);
        assert_eq!(span.offset_ratio, 0.45);
    }

    #[test]
    fn looped_waveform_click_resolves_to_playmark_span_when_selected() {
        let mut state = gui_state_for_span_tests();
        state.loop_playback = true;
        state
            .waveform
            .apply_interaction(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.25,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::UpdateSelection {
                visible_ratio: 0.60,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::FinishSelection {
                visible_ratio: 0.60,
            });

        let inside_span = state.resolve_playback_span(0.45, 1.0, None);
        assert_eq!(inside_span.start_ratio, 0.25);
        assert_eq!(inside_span.end_ratio, 0.60);
        assert_eq!(inside_span.offset_ratio, 0.45);

        let outside_span = state.resolve_playback_span(0.85, 1.0, None);
        assert_eq!(outside_span.start_ratio, 0.25);
        assert_eq!(outside_span.end_ratio, 0.60);
        assert_eq!(outside_span.offset_ratio, 0.25);
    }

    #[test]
    fn normalize_wav_file_in_place_scales_loaded_sample_peak() {
        let root = std::env::temp_dir().join(format!(
            "wavecrate-default-gui-normalize-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("quiet.wav");
        write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

        super::normalize_wav_file_in_place(&path).expect("normalize wav");

        let samples = read_test_wav_f32(&path);
        let peak = samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        assert!((peak - 1.0).abs() < 0.000_001, "peak was {peak}");
        assert!(samples.iter().all(|sample| sample.is_finite()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sample_selection_loads_selected_file_into_waveform() {
        let mut state = GuiAppState {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: super::FolderBrowserState::load_default(),
            waveform: super::WaveformState::synthetic_for_tests(),
            sample_status: String::new(),
            worker_sender: mpsc::channel().0,
            worker_receiver: None,
            next_task_id: 1,
            sample_load_task: ui::LatestTask::new(),
            folder_progress: None,
            progress_tick: 0.0,
            waveform_loading_progress: 0.0,
            waveform_loading_target_progress: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            persisted_settings: super::AppSettingsCore::default(),
            audio_settings_open: false,
            audio_backend_dropdown_open: false,
            audio_output_dropdown_open: false,
            audio_sample_rate_dropdown_open: false,
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            native_file_drop_hover: None,
        };
        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");

        let mut context = ui::UpdateContext::default();
        state.apply_message(
            super::GuiMessage::SelectSampleWithModifiers {
                path: sample_path.clone(),
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert_eq!(
            state.waveform_loading_label.as_deref(),
            Some("portal_SS_kick_003.wav")
        );
        let ticket = state.sample_load_task.active().expect("sample load queued");
        state.apply_message(
            super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
                ticket,
                output: super::SampleLoadResult {
                    path: sample_path.clone(),
                    result: super::WaveformState::load_path(sample_path.clone().into()),
                },
            }),
            &mut context,
        );

        assert_eq!(
            state.folder_browser.selected_file_id(),
            Some(sample_path.as_str())
        );
        assert_eq!(state.waveform.file_name(), "portal_SS_kick_003.wav");
        assert_eq!(state.waveform_loading_label, None);
        assert!(state.waveform.frames() > 0);
        assert!(state.sample_status.contains("portal_SS_kick_003.wav"));
    }

    #[test]
    fn play_selected_sample_uses_active_playmark_selection_span() {
        let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
            return;
        };
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_player = Some(player);
        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
        state.waveform =
            super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
        state
            .waveform
            .apply_interaction(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.25,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::UpdateSelection {
                visible_ratio: 0.60,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::FinishSelection {
                visible_ratio: 0.60,
            });
        state.loop_playback = true;

        let mut context = ui::UpdateContext::default();
        state.play_selected_sample(&mut context);

        assert!(state.waveform.is_playing());
        assert_eq!(state.waveform.play_mark_ratio(), Some(0.25));
        assert_eq!(state.current_playback_span, Some((0.25, 0.6)));
        assert!(
            state
                .audio_player
                .as_ref()
                .is_some_and(|player| player.is_looping())
        );
        let progress = state
            .audio_player
            .as_ref()
            .and_then(|player| player.progress())
            .expect("playback progress");
        assert!(
            (0.24..=0.35).contains(&progress),
            "spacebar playback should start inside the playmark selection, got {progress}"
        );
    }

    #[test]
    fn bottom_status_bar_reports_selected_sample_count() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let empty_frame = radiant::runtime::UiSurface::new(
            super::status_bar::bottom_status_bar(&state).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 30.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        assert!(frame_has_text(&empty_frame, "0 samples"));
        assert!(!frame_has_text(&empty_frame, "1 sample"));

        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
        state.folder_browser.select_file(sample_path);
        let selected_frame = radiant::runtime::UiSurface::new(
            super::status_bar::bottom_status_bar(&state).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 30.0)),
            &radiant::theme::ThemeTokens::default(),
        );

        assert!(frame_has_text(&selected_frame, "1 sample"));
    }

    #[test]
    fn bottom_status_progress_bar_paints_without_text_chrome() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.folder_progress = Some(super::FolderScanProgress {
            task_id: 7,
            source_id: String::from("assets"),
            label: String::from("Assets"),
            phase: String::from("Scanning"),
            completed: 2,
            total: 5,
            detail: String::from("kick.wav"),
        });
        let frame = radiant::runtime::UiSurface::new(
            super::status_bar::worker_progress_bar(&state).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0)),
            &radiant::theme::ThemeTokens::default(),
        );

        let fills = frame
            .paint_plan
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count();
        assert_eq!(fills, 2);
        assert!(
            frame
                .paint_plan
                .primitives
                .iter()
                .all(|primitive| !matches!(primitive, PaintPrimitive::StrokeRect(_)))
        );
    }

    #[test]
    fn bottom_status_progress_bar_click_opens_job_details() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0));
        let mut progress = super::status_bar::StatusProgressBar::determinate(0.4);
        assert_eq!(
            progress.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(90.0, 5.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            ),
            None
        );

        let output = progress
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(90.0, 5.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("progress bar click should activate details");
        assert_eq!(
            output.typed_ref::<super::GuiMessage>(),
            Some(&super::GuiMessage::ToggleJobDetails)
        );
    }

    #[test]
    fn bottom_status_progress_bar_shows_indeterminate_fill_for_unknown_totals() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.progress_tick = 0.5;
        state.folder_progress = Some(super::FolderScanProgress {
            task_id: 7,
            source_id: String::from("assets"),
            label: String::from("Assets"),
            phase: String::from("Scanning"),
            completed: 128,
            total: 0,
            detail: String::from("kick.wav"),
        });
        let frame = radiant::runtime::UiSurface::new(
            super::status_bar::worker_progress_bar(&state).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0)),
            &radiant::theme::ThemeTokens::default(),
        );

        let fills = frame
            .paint_plan
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count();
        assert_eq!(fills, 2);
        assert!(
            frame
                .paint_plan
                .primitives
                .iter()
                .all(|primitive| !matches!(primitive, PaintPrimitive::StrokeRect(_)))
        );
    }

    #[test]
    fn job_details_popover_reports_active_scan_progress() {
        let progress = super::FolderScanProgress {
            task_id: 7,
            source_id: String::from("assets"),
            label: String::from("Assets"),
            phase: String::from("Scanning"),
            completed: 2,
            total: 5,
            detail: String::from("kick.wav"),
        };
        let frame = radiant::runtime::UiSurface::new(
            super::status_bar::job_details_popover(&progress).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(360.0, 180.0)),
            &radiant::theme::ThemeTokens::default(),
        );

        assert!(frame_has_text(&frame, "Job Details"));
        assert!(frame_has_text(&frame, "Type: Scanning"));
        assert!(frame_has_text(&frame, "Progress: 2/5"));
        assert!(frame_has_text(&frame, "Current: kick.wav"));
    }

    #[test]
    fn looped_playback_retargets_when_playmark_selection_is_created_and_resized() {
        let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
            return;
        };
        let mut state = gui_state_for_span_tests();
        state.audio_player = Some(player);
        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
        state.waveform =
            super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
        state.loop_playback = true;
        state
            .start_playback_current_span(0.0, 1.0)
            .expect("full sample loop starts");
        assert_player_progress_inside_span(&state, 0.0, 1.0);

        let mut context = ui::UpdateContext::default();
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.25,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
                visible_ratio: 0.60,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
                visible_ratio: 0.60,
            }),
            &mut context,
        );

        assert_playback_span_state(&state, 0.25, 0.60);
        assert_player_progress_inside_span(&state, 0.25, 0.60);
        assert!(
            state
                .audio_player
                .as_ref()
                .is_some_and(|player| player.is_looping())
        );

        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::BeginSelectionResize {
                kind: WaveformSelectionKind::Play,
                edge: WaveformSelectionEdge::Start,
                visible_ratio: 0.25,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
                visible_ratio: 0.10,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
                visible_ratio: 0.10,
            }),
            &mut context,
        );

        assert_playback_span_state(&state, 0.10, 0.60);
        assert_player_progress_inside_span(&state, 0.10, 0.60);
    }

    fn assert_playback_span_state(state: &GuiAppState, expected_start: f32, expected_end: f32) {
        let (start, end) = state
            .current_playback_span
            .expect("current playback span should be set");
        assert!(
            (start - expected_start).abs() < 0.001,
            "start {start}, expected {expected_start}"
        );
        assert!(
            (end - expected_end).abs() < 0.001,
            "end {end}, expected {expected_end}"
        );
    }

    fn assert_player_progress_inside_span(state: &GuiAppState, start: f32, end: f32) {
        let progress = state
            .audio_player
            .as_ref()
            .and_then(|player| player.progress())
            .expect("audio player progress should be available");
        assert!(
            progress >= start - 0.02 && progress <= end + 0.02,
            "progress {progress}, expected inside {start}..={end}"
        );
    }

    #[test]
    fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
        for icon in [
            super::ToolbarIcon::Loop,
            super::ToolbarIcon::Play,
            super::ToolbarIcon::Stop,
        ] {
            assert!(radiant::gui::svg::SvgIcon::from_svg(icon.svg()).is_some());
            let frame = radiant::runtime::UiSurface::new(
                super::toolbar_icon_button(101, icon, true, false).into_node(),
            )
            .frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(28.0, 24.0)),
                &radiant::theme::ThemeTokens::default(),
            );
            assert!(
                frame
                    .paint_plan
                    .primitives
                    .iter()
                    .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
                "toolbar icon should paint as a retained Radiant SVG"
            );
        }
    }

    #[test]
    fn toolbar_icon_button_routes_transport_message_through_radiant_builder() {
        let surface = radiant::runtime::UiSurface::new(
            super::toolbar_icon_button(101, super::ToolbarIcon::Loop, true, false).into_node(),
        );

        assert_eq!(
            surface.dispatch_widget_output(
                101,
                radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate),
            ),
            Some(super::GuiMessage::ToggleLoopPlayback)
        );
    }

    #[test]
    fn native_file_hover_over_waveform_tracks_supported_state() {
        let root = temp_gui_root("wavecrate-native-file-hover");
        let wav = root.join("kick.wav");
        let txt = root.join("note.txt");
        write_test_wav_i16(&wav, &[0, 100]);
        fs::write(&txt, "not audio").expect("write text");
        let mut state = gui_state_for_span_tests();
        state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
        let mut context = ui::UpdateContext::default();

        state.apply_native_file_drop(
            NativeFileDrop::hover(
                wav.clone(),
                Some(Point::new(8.0, 8.0)),
                Some(super::WAVEFORM_WIDGET_ID),
            ),
            &mut context,
        );
        assert_eq!(
            state.native_file_drop_hover,
            Some(super::NativeFileDropHover {
                path: wav.clone(),
                supported: true,
            })
        );

        state.apply_native_file_drop(
            NativeFileDrop::hover(
                txt.clone(),
                Some(Point::new(8.0, 8.0)),
                Some(super::WAVEFORM_WIDGET_ID),
            ),
            &mut context,
        );
        assert_eq!(
            state.native_file_drop_hover,
            Some(super::NativeFileDropHover {
                path: txt,
                supported: false,
            })
        );

        state.apply_native_file_drop(
            NativeFileDrop::cancel(Some(Point::new(8.0, 8.0)), Some(super::WAVEFORM_WIDGET_ID)),
            &mut context,
        );
        assert_eq!(state.native_file_drop_hover, None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_file_drop_on_waveform_copies_into_selected_folder_and_queues_load() {
        let root = temp_gui_root("wavecrate-native-file-drop-root");
        let external_root = temp_gui_root("wavecrate-native-file-drop-external");
        let loops = root.join("loops");
        fs::create_dir_all(&loops).expect("create loops");
        let source = external_root.join("kick.wav");
        write_test_wav_i16(&source, &[0, 100, -100]);
        let mut state = gui_state_for_span_tests();
        state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
        state
            .folder_browser
            .apply_message(super::FolderBrowserMessage::ActivateFolder(
                loops.display().to_string(),
            ));
        let mut context = ui::UpdateContext::default();

        state.apply_native_file_drop(
            NativeFileDrop::dropped(
                source,
                Some(Point::new(8.0, 8.0)),
                Some(super::WAVEFORM_WIDGET_ID),
            ),
            &mut context,
        );

        let copied = loops.join("kick.wav");
        let copied_id = copied.display().to_string();
        assert!(copied.is_file());
        assert_eq!(
            state.folder_browser.selected_file_id(),
            Some(copied_id.as_str())
        );
        assert_eq!(state.waveform_loading_label.as_deref(), Some("kick.wav"));
        assert!(state.sample_load_task.active().is_some());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(external_root);
    }

    fn temp_gui_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_test_wav_i16(path: &std::path::Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for sample in samples {
            writer.write_sample(*sample).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    fn read_test_wav_f32(path: &std::path::Path) -> Vec<f32> {
        let mut reader = hound::WavReader::open(path).expect("open wav");
        reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .expect("read samples")
    }

    #[test]
    fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 22.0));
        let mut hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);

        assert_eq!(
            hit_target.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(24.0, 10.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            ),
            None
        );

        let mut refreshed_hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);
        refreshed_hit_target.common_mut().state = hit_target.common().state;
        let output = refreshed_hit_target
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 10.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers {
                        command: true,
                        shift: true,
                        ..Default::default()
                    },
                },
            )
            .expect("sample row should activate after a frame refresh");

        assert_eq!(
            output.typed_ref::<super::sample_browser_view::SampleFileHitMessage>(),
            Some(&super::sample_browser_view::SampleFileHitMessage::Activate(
                PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                }
            ))
        );
        assert!(!refreshed_hit_target.common().state.pressed);
    }

    #[test]
    fn top_status_bar_replaces_text_labels_with_volume_slider_and_audio_pill() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_output_resolved = Some(super::ResolvedOutput {
            host_id: String::from("wasapi"),
            device_name: String::from("Studio"),
            sample_rate: 48_000,
            buffer_size_frames: None,
            channel_count: 2,
            used_fallback: false,
        });
        let frame = radiant::runtime::UiSurface::new(super::top_status_bar(&state).into_node())
            .frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 30.0)),
                &radiant::theme::ThemeTokens::default(),
            );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let slider_fills = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill)
                    if fill.widget_id == super::VOLUME_SLIDER_ID
                        && fill.rect.width() > 0.0
                        && fill.rect.height() > 0.0 =>
                {
                    Some(fill)
                }
                _ => None,
            })
            .count();

        assert!(!texts.iter().any(|text| text == "Wavecrate"));
        assert!(!texts.iter().any(|text| text == "Wavecrate GUI"));
        assert!(!texts.iter().any(|text| text == "ready"));
        assert!(texts.iter().any(|text| text == "Audio"), "{texts:?}");
        assert!(slider_fills >= 2, "expected track and fill rects");
    }

    #[test]
    fn volume_slider_drag_emits_normalized_volume() {
        assert_eq!(
            radiant::runtime::UiSurface::new(
                super::audio_settings::volume_slider(0.25).into_node(),
            )
            .dispatch_widget_output(
                super::VOLUME_SLIDER_ID,
                radiant::widgets::WidgetOutput::typed(
                    radiant::widgets::SliderMessage::ValueChanged { value: 0.75 },
                ),
            ),
            Some(super::GuiMessage::SetVolume(0.75))
        );
    }

    #[test]
    fn default_gui_volume_state_clamps() {
        let mut state = GuiAppState::load_default().expect("default state loads");

        state.set_volume(1.5);
        assert_eq!(state.volume, 1.0);

        state.set_volume(-0.5);
        assert_eq!(state.volume, 0.0);
    }

    #[test]
    fn audio_engine_pill_activates_settings_toggle() {
        let mut pill = super::audio_settings::AudioEnginePill::new(String::from("48 kHz"), false);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(66.0, 18.0));
        assert!(
            pill.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(24.0, 8.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .is_none()
        );
        let output = pill
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 8.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("audio pill should activate");

        assert_eq!(
            output.typed_ref::<super::GuiMessage>(),
            Some(&super::GuiMessage::ToggleAudioSettings)
        );
    }

    #[test]
    fn audio_settings_snapshot_uses_cached_device_options() {
        let mut state = gui_state_for_span_tests();
        state.audio_hosts = vec![super::AudioHostSummary {
            id: String::from("cached-host"),
            label: String::from("Cached Host"),
            is_default: true,
        }];

        let snapshot = super::audio_settings::AudioSettingsSnapshot::from_app_state(&state);

        assert_eq!(snapshot.audio_hosts.len(), 1);
        assert_eq!(snapshot.audio_hosts[0].id, "cached-host");
    }

    #[test]
    fn audio_engine_detail_distinguishes_selected_host_from_runtime_fallback() {
        let mut state = gui_state_for_span_tests();
        state.audio_output_config.host = Some(String::from("asio"));
        state.audio_hosts = vec![
            super::AudioHostSummary {
                id: String::from("wasapi"),
                label: String::from("WASAPI"),
                is_default: true,
            },
            super::AudioHostSummary {
                id: String::from("asio"),
                label: String::from("ASIO"),
                is_default: false,
            },
        ];
        state.audio_output_resolved = Some(super::ResolvedOutput {
            host_id: String::from("wasapi"),
            device_name: String::from("Studio"),
            sample_rate: 48_000,
            buffer_size_frames: None,
            channel_count: 2,
            used_fallback: true,
        });

        assert_eq!(
            state.audio_engine_detail_label(),
            "ASIO selected | using WASAPI | Studio | 48 kHz"
        );
    }

    #[test]
    fn audio_sample_rate_label_matches_status_chip_format() {
        assert_eq!(super::format_sample_rate_label(48_000), "48 kHz");
        assert_eq!(super::format_sample_rate_label(44_100), "44.1 kHz");
        assert_eq!(super::format_sample_rate_label(960), "960 Hz");
    }

    #[test]
    fn audio_settings_popover_stays_output_only() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_error = None;
        state.audio_hosts = vec![super::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        }];
        state.audio_devices = vec![super::AudioDeviceSummary {
            host_id: String::from("asio"),
            name: String::from("Studio Out"),
            is_default: true,
        }];
        state.audio_sample_rates = vec![44_100, 48_000];
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            !texts.iter().any(|text| text == "Audio Engine"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == "Backend"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Output"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "Clear Rebuildable Caches"),
            "{texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.contains("Input")),
            "{texts:?}"
        );
    }

    #[test]
    fn audio_backend_dropdown_renders_expanded_host_options() {
        let mut state = gui_state_for_span_tests();
        state.audio_settings_error = None;
        state.audio_backend_dropdown_open = true;
        state.audio_hosts = vec![
            super::AudioHostSummary {
                id: String::from("wasapi"),
                label: String::from("WASAPI"),
                is_default: true,
            },
            super::AudioHostSummary {
                id: String::from("asio"),
                label: String::from("ASIO"),
                is_default: false,
            },
        ];
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            texts.iter().any(|text| text == "System default"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "WASAPI (default)"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == "ASIO"), "{texts:?}");
    }

    #[test]
    fn audio_output_dropdown_renders_expanded_device_options() {
        let mut state = gui_state_for_span_tests();
        state.audio_settings_error = None;
        state.audio_output_dropdown_open = true;
        state.audio_devices = vec![super::AudioDeviceSummary {
            host_id: String::from("asio"),
            name: String::from("Studio Out"),
            is_default: true,
        }];
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(texts.iter().any(|text| text == "Host default"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "Studio Out (default)"),
            "{texts:?}"
        );
    }

    #[test]
    fn audio_sample_rate_dropdown_renders_expanded_rate_options() {
        let mut state = gui_state_for_span_tests();
        state.audio_settings_error = None;
        state.audio_sample_rate_dropdown_open = true;
        state.audio_sample_rates = vec![44_100, 48_000];
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            texts.iter().any(|text| text == "Device default"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == "44.1 kHz"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
    }

    #[test]
    fn audio_backend_dropdown_overlay_does_not_reflow_later_sections() {
        let mut state = gui_state_for_span_tests();
        state.audio_settings_error = None;
        state.audio_hosts = vec![
            super::AudioHostSummary {
                id: String::from("wasapi"),
                label: String::from("WASAPI"),
                is_default: true,
            },
            super::AudioHostSummary {
                id: String::from("asio"),
                label: String::from("ASIO"),
                is_default: false,
            },
        ];

        state.audio_backend_dropdown_open = false;
        let closed =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        state.audio_backend_dropdown_open = true;
        let open =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );

        let text_top = |frame: &radiant::runtime::SurfaceFrame, label: &str| {
            frame
                .paint_plan
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    PaintPrimitive::Text(text) if text.text.as_str() == label => {
                        Some(text.rect.min.y)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("expected text {label}"))
        };
        let text_index = |frame: &radiant::runtime::SurfaceFrame, label: &str| {
            frame
                .paint_plan
                .primitives
                .iter()
                .position(|primitive| match primitive {
                    PaintPrimitive::Text(text) => text.text.as_str() == label,
                    _ => false,
                })
                .unwrap_or_else(|| panic!("expected text {label}"))
        };

        assert_eq!(text_top(&closed, "Output"), text_top(&open, "Output"));
        assert_eq!(
            text_top(&closed, "Sample Rate"),
            text_top(&open, "Sample Rate")
        );
        assert!(text_top(&open, "WASAPI (default)") > text_top(&open, "Output"));
        assert!(text_index(&open, "WASAPI (default)") > text_index(&open, "Output"));
    }

    #[test]
    fn audio_backend_dropdown_toggle_and_close_are_ui_only() {
        let mut state = gui_state_for_span_tests();

        state.apply_message(
            super::GuiMessage::ToggleAudioBackendDropdown,
            &mut ui::UpdateContext::default(),
        );
        assert!(state.audio_backend_dropdown_open);

        state.apply_message(
            super::GuiMessage::CloseAudioSettingsDropdowns,
            &mut ui::UpdateContext::default(),
        );
        assert!(!state.audio_backend_dropdown_open);

        state.apply_message(
            super::GuiMessage::ToggleAudioBackendDropdown,
            &mut ui::UpdateContext::default(),
        );
        assert!(state.audio_backend_dropdown_open);

        state.apply_message(
            super::GuiMessage::ToggleAudioOutputDropdown,
            &mut ui::UpdateContext::default(),
        );
        assert!(!state.audio_backend_dropdown_open);
        assert!(state.audio_output_dropdown_open);

        state.apply_message(
            super::GuiMessage::ToggleAudioSampleRateDropdown,
            &mut ui::UpdateContext::default(),
        );
        assert!(!state.audio_output_dropdown_open);
        assert!(state.audio_sample_rate_dropdown_open);

        state.apply_message(
            super::GuiMessage::CloseAudioSettingsDropdowns,
            &mut ui::UpdateContext::default(),
        );
        assert!(!state.audio_sample_rate_dropdown_open);

        state.apply_message(
            super::GuiMessage::CloseAudioSettings,
            &mut ui::UpdateContext::default(),
        );
        assert!(!state.audio_backend_dropdown_open);
        assert!(!state.audio_output_dropdown_open);
        assert!(!state.audio_sample_rate_dropdown_open);
    }

    #[test]
    fn clear_rebuildable_caches_action_removes_cache_payloads_only() {
        if std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
            || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
        {
            return;
        }
        let base = tempfile::tempdir().expect("create config base");
        let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = wavecrate::app_dirs::PersistenceProfileGuard::live();
        let waveform_cache = wavecrate::app_dirs::waveform_cache_dir().expect("waveform cache dir");
        let cache_payload = waveform_cache.join("cached.bin");
        std::fs::write(&cache_payload, b"cache").expect("write cache payload");
        let handoff_dir = wavecrate::app_dirs::handoff_staging_dir().expect("handoff staging dir");
        let handoff_payload = handoff_dir.join("clip.wav");
        std::fs::write(&handoff_payload, b"clip").expect("write handoff payload");
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.sample_status = String::from("ready");

        state.apply_message(
            super::GuiMessage::ClearRebuildableCaches,
            &mut ui::UpdateContext::default(),
        );

        assert!(!cache_payload.exists());
        assert!(handoff_payload.exists());
        assert_eq!(state.audio_settings_error, None);
        assert!(
            state.sample_status.contains("Rebuildable caches cleared"),
            "{}",
            state.sample_status
        );
    }

    #[test]
    fn audio_settings_popover_opens_as_centered_floating_window() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_error = None;
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        assert!(
            !frame.paint_plan.primitives.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::Text(text) if text.text.as_str() == "Audio Engine"
                )
            }),
            "audio settings should rely on the native window title"
        );
        let backend_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str() == "Backend" => Some(text.rect),
                _ => None,
            })
            .expect("audio settings backend label paints");

        assert!(
            (66.0..=74.0).contains(&backend_rect.min.x),
            "{backend_rect:?}"
        );
        assert!(
            (41.0..=49.0).contains(&backend_rect.min.y),
            "{backend_rect:?}"
        );
    }

    #[test]
    fn audio_settings_window_does_not_add_full_height_panel_chrome() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;
        let frame = radiant::runtime::UiSurface::new(super::view(&mut state).into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 540.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        let audio_panel_fills = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill)
                    if fill.widget_id == 0
                        && fill.rect.min.x >= 250.0
                        && fill.rect.max.x <= 710.0
                        && fill.rect.width() >= 300.0 =>
                {
                    Some(fill.rect)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            audio_panel_fills
                .iter()
                .all(|rect| rect.height() <= super::AUDIO_SETTINGS_POPUP_HEIGHT + 1.0),
            "{audio_panel_fills:?}"
        );
    }

    #[test]
    fn audio_settings_window_does_not_block_waveform_selection_messages() {
        let mut state = gui_state_for_span_tests();
        state.audio_settings_open = true;
        let mut context = ui::UpdateContext::default();

        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.45,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
                visible_ratio: 0.65,
            }),
            &mut context,
        );
        state.apply_message(
            super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
                visible_ratio: 0.65,
            }),
            &mut context,
        );

        assert_eq!(state.waveform.play_mark_ratio(), Some(0.45));
        assert_eq!(
            state.waveform.play_selection(),
            Some(wavecrate::selection::SelectionRange::new(0.45, 0.65))
        );
    }

    #[test]
    fn default_folder_browser_loads_assets_root() {
        let browser = super::FolderBrowserState::load_default();
        assert!(browser.root_path().ends_with("assets"));
        assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
        assert!(
            browser
                .selected_files()
                .iter()
                .any(|file| file.name == "portal_SS_kick_003.wav")
        );
        assert!(
            browser
                .selected_audio_files()
                .iter()
                .any(|file| file.name == "portal_SS_kick_003.wav")
        );
    }

    #[test]
    fn default_gui_loads_persisted_sources_and_audio_output() {
        let config_base = tempfile::tempdir().expect("config base");
        let _base_guard =
            wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
        let source_root = tempfile::tempdir().expect("source root");
        let source_id = wavecrate::sample_sources::SourceId::from_string("source_id::gui-test");
        wavecrate::sample_sources::config::save(&super::AppConfig {
            sources: vec![wavecrate::sample_sources::SampleSource::new_with_id(
                source_id,
                source_root.path().to_path_buf(),
            )],
            core: super::AppSettingsCore {
                audio_output: super::AudioOutputConfig {
                    host: Some(String::from("test-host")),
                    device: Some(String::from("Test Device")),
                    sample_rate: Some(48_000),
                    buffer_size: Some(256),
                },
                volume: 0.42,
                ..super::AppSettingsCore::default()
            },
        })
        .expect("seed config");

        let state = GuiAppState::load_default().expect("default state loads persisted config");

        assert_eq!(state.folder_browser.root_path(), source_root.path());
        assert_eq!(state.audio_output_config.host.as_deref(), Some("test-host"));
        assert_eq!(
            state.audio_output_config.device.as_deref(),
            Some("Test Device")
        );
        assert_eq!(state.audio_output_config.sample_rate, Some(48_000));
        assert!((state.volume - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn default_gui_saves_sources_and_audio_output_to_app_config() {
        let config_base = tempfile::tempdir().expect("config base");
        let _base_guard =
            wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
        let source_root = tempfile::tempdir().expect("source root");
        let mut state = gui_state_for_span_tests();
        state.audio_output_config = super::AudioOutputConfig {
            host: Some(String::from("wasapi")),
            device: Some(String::from("Interface")),
            sample_rate: Some(96_000),
            buffer_size: None,
        };
        state.volume = 0.5;

        let request = state
            .folder_browser
            .begin_add_source_path(source_root.path().to_path_buf(), 100)
            .expect("new source requests scan");
        let result = super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
        state.finish_folder_scan(result);

        let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].root, source_root.path());
        assert_eq!(loaded.core.audio_output.host.as_deref(), Some("wasapi"));
        assert_eq!(
            loaded.core.audio_output.device.as_deref(),
            Some("Interface")
        );
        assert_eq!(loaded.core.audio_output.sample_rate, Some(96_000));
        assert!((loaded.core.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn sample_browser_frame_paints_column_and_file_text() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let surface = super::sample_browser(&mut state).into_node();
        let frame = radiant::runtime::UiSurface::new(surface).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        let texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            texts.iter().any(|text| text.starts_with("Name")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("portal_SS_")),
            "{texts:?}"
        );
    }

    #[test]
    fn sample_browser_rows_match_keyboard_scroll_stride() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let surface = super::sample_browser(&mut state).into_node();
        let frame = radiant::runtime::UiSurface::new(surface).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        let mut row_tops = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str().starts_with("portal_SS_") => {
                    Some(text.rect.min.y)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        row_tops.sort_by(|a, b| a.total_cmp(b));
        row_tops.dedup_by(|a, b| (*a - *b).abs() < 0.5);

        assert!(row_tops.len() >= 2, "{row_tops:?}");
        assert!(
            row_tops
                .windows(2)
                .all(|pair| ((pair[1] - pair[0]) - super::SAMPLE_BROWSER_ROW_HEIGHT).abs() < 0.5),
            "{row_tops:?}"
        );
    }

    #[test]
    fn sample_browser_keyboard_scroll_keeps_two_context_rows() {
        assert_eq!(super::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, 2);
        assert_eq!(super::SAMPLE_BROWSER_ROW_HEIGHT, 22.0);
    }

    #[test]
    fn selected_sample_browser_row_paints_strong_fill_and_left_marker() {
        let widget = super::sample_browser_view::SampleFileHitTarget::new(true);
        let bounds = Rect::from_min_size(Point::new(12.0, 8.0), Vector2::new(240.0, 22.0));
        let mut primitives = Vec::new();
        widget.append_paint(
            &mut primitives,
            bounds,
            &Default::default(),
            &radiant::theme::ThemeTokens::default(),
        );
        let fills = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(fills.iter().any(|fill| fill.rect == bounds
            && fill.color
                == super::Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 120,
                }));
        assert!(fills.iter().any(|fill| {
            fill.color
                == super::Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 245,
                }
                && fill.rect.width() <= 3.5
        }));
    }

    #[test]
    fn sample_browser_row_hover_paints_bright_background_without_marker() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 22.0));
        let mut hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);

        assert_eq!(
            hit_target.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(20.0, 10.0),
                },
            ),
            None
        );

        let mut primitives = Vec::new();
        hit_target.append_paint(
            &mut primitives,
            bounds,
            &Default::default(),
            &radiant::theme::ThemeTokens::default(),
        );
        let fills = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(fills.len(), 1, "{fills:?}");
        assert_eq!(fills[0].rect, bounds);
        assert_eq!(
            fills[0].color,
            super::Rgba8 {
                r: 255,
                g: 108,
                b: 88,
                a: 155,
            }
        );
    }

    #[test]
    fn full_gui_frame_places_sample_browser_text_inside_visible_area() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let surface = super::view(&mut state).into_node();
        let frame = radiant::runtime::UiSurface::new(surface).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1517.0, 758.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        let sample_texts = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Text(text)
                    if text.text.as_str() == "Name"
                        || text.text.as_str().starts_with("portal_SS_") =>
                {
                    Some((text.text.as_str().to_string(), text.rect, text.baseline))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(!sample_texts.is_empty(), "{sample_texts:?}");
        assert!(
            sample_texts.iter().any(|(_, rect, baseline)| {
                rect.width() > 20.0
                    && rect.height() >= 10.0
                    && rect.min.x >= 280.0
                    && rect.min.y >= 320.0
                    && rect.max.y <= 730.0
                    && baseline.is_some()
            }),
            "{sample_texts:?}"
        );
    }

    #[test]
    fn waveform_loading_visual_paints_full_height_gray_fill_without_chrome() {
        let frame = radiant::runtime::UiSurface::new(
            super::waveform_loading_visual("kick.wav", 0.25).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 172.0)),
            &radiant::theme::ThemeTokens::default(),
        );

        let fill_rects = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(rect) => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(fill_rects.iter().any(|fill| {
            (fill.rect.width() - 180.0).abs() < 0.01
                && (fill.rect.height() - 172.0).abs() < 0.01
                && fill.rect.min.x == 0.0
                && fill.rect.min.y == 0.0
                && fill.color.r == 174
                && fill.color.g == 178
                && fill.color.b == 181
        }));
        assert!(
            frame
                .paint_plan
                .primitives
                .iter()
                .all(|primitive| !matches!(
                    primitive,
                    PaintPrimitive::StrokeRect(_) | PaintPrimitive::Text(_)
                ))
        );
    }

    fn frame_has_text(frame: &ui::SurfaceFrame, expected: &str) -> bool {
        frame
            .paint_plan
            .primitives
            .iter()
            .any(|primitive| match primitive {
                PaintPrimitive::Text(text) => text.text.as_str() == expected,
                _ => false,
            })
    }
}
