//! Default Wavecrate GUI application built on Radiant's current public API.

use radiant::gui::types::{ImageRgba, Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{
    NativeRunOptions, NativeTextOptions, PaintFillRect, PaintImage, PaintPrimitive,
    PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, PointerModifiers,
    ScrollbarMessage, TextInputWidget, TextWrap, Widget, WidgetCommon, WidgetInput, WidgetOutput,
    WidgetSizing,
};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::{
    ffi::OsString,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant, SystemTime},
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
    available_devices, available_hosts, supported_sample_rates,
};
use wavecrate::gui::svg::{parse_svg_document, point_in_svg_shapes};
use wavecrate::gui_runtime::wavecrate_ui_font_path;
use wavecrate::logging::{self, ActionDebugEvent, emit_action_debug_event};

mod folder_browser;
mod waveform;
use folder_browser::{
    FileColumn, FileEntry, FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch,
    FolderScanProgress, FolderScanRequest, FolderScanResult,
};
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
const DEFAULT_VOLUME: f32 = 1.0;
const VOLUME_SLIDER_ID: u64 = 31_000;
const VOLUME_SLIDER_WIDTH: f32 = 92.0;
const VOLUME_SLIDER_HEIGHT: f32 = 14.0;
const AUDIO_ENGINE_PILL_ID: u64 = 31_100;
const AUDIO_ENGINE_PILL_WIDTH: f32 = 66.0;
const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
const AUDIO_SETTINGS_MODAL_BLOCKER_ID: u64 = 31_150;
const AUDIO_SETTINGS_MODAL_PILL_ID: u64 = 31_151;
const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 360.0;
const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 316.0;
const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
const DRAG_PREVIEW_HEIGHT: f32 = 24.0;
const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;
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
    DragSampleFile {
        path: String,
        drag: DragHandleMessage,
    },
    ExternalDragCompleted(Result<ui::ExternalDragOutcome, String>),
    SampleLoadFinished(SampleLoadResult),
    PlaySelectedSample,
    StopPlayback,
    ToggleLoopPlayback,
    SetVolume(f32),
    ToggleAudioSettings,
    CloseAudioSettings,
    SetAudioOutputHost(Option<String>),
    SetAudioOutputDevice(Option<String>),
    SetAudioOutputSampleRate(Option<u32>),
    NormalizeSelectedSamples,
    FocusRenameInput(u64),
    DeleteSelectedItem,
    ExtractPlaymarkedRange,
    ClearExtractionHistory,
    NavigateBrowser {
        delta: i32,
        extend: bool,
    },
    SelectAllSamples,
    CollapseSelectedFolder,
    ExpandSelectedFolder,
    Waveform(WaveformInteraction),
    Frame,
}

#[derive(Clone, Debug)]
struct SampleLoadResult {
    task_id: u64,
    path: String,
    result: Result<WaveformState, String>,
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
            && self.path == other.path
            && self.result.as_ref().err() == other.result.as_ref().err()
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
    next_sample_task_id: u64,
    pending_sample_task_id: Option<u64>,
    folder_progress: Option<FolderScanProgress>,
    progress_tick: f32,
    audio_player: Option<AudioPlayer>,
    loop_playback: bool,
    volume: f32,
    audio_output_config: AudioOutputConfig,
    audio_output_resolved: Option<ResolvedOutput>,
    audio_hosts: Vec<AudioHostSummary>,
    audio_devices: Vec<AudioDeviceSummary>,
    audio_sample_rates: Vec<u32>,
    audio_settings_open: bool,
    audio_settings_error: Option<String>,
    current_playback_span: Option<(f32, f32)>,
}

impl GuiAppState {
    fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let (worker_sender, worker_receiver) = mpsc::channel();
        let mut state = Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: FolderBrowserState::load_default(),
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Select a sample to load"),
            worker_sender,
            worker_receiver: Some(worker_receiver),
            next_task_id: 1,
            next_sample_task_id: 1,
            pending_sample_task_id: None,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: DEFAULT_VOLUME,
            audio_output_config: AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            audio_settings_open: false,
            audio_settings_error: None,
            current_playback_span: None,
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

    fn apply_message(&mut self, message: GuiMessage, context: &mut ui::UpdateContext<GuiMessage>) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource) => {
                self.add_source_from_dialog(context);
            }
            GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(id)) => {
                self.select_source(id, context);
            }
            GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected) => {
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
            GuiMessage::FolderBrowser(FolderBrowserMessage::BeginCreateSubfolder) => {
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
            GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message)) => {
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
            GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(folder_id)) => {
                self.drop_browser_drag_on_folder(folder_id, context);
            }
            GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(folder_id, drag)) => {
                self.drag_folder(folder_id, drag, context);
            }
            GuiMessage::FolderBrowser(message) => self.folder_browser.apply_message(message),
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
                self.select_sample_with_modifiers(path, modifiers, context);
            }
            GuiMessage::DragSampleFile { path, drag } => {
                self.drag_sample_file(path, drag, context);
            }
            GuiMessage::ExternalDragCompleted(result) => self.external_drag_completed(result),
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result),
            GuiMessage::PlaySelectedSample => self.play_selected_sample(context),
            GuiMessage::StopPlayback => self.stop_playback(),
            GuiMessage::ToggleLoopPlayback => self.toggle_loop_playback(),
            GuiMessage::SetVolume(volume) => self.set_volume(volume),
            GuiMessage::ToggleAudioSettings => self.toggle_audio_settings(),
            GuiMessage::CloseAudioSettings => {
                self.audio_settings_open = false;
            }
            GuiMessage::SetAudioOutputHost(host) => self.set_audio_output_host(host),
            GuiMessage::SetAudioOutputDevice(device) => self.set_audio_output_device(device),
            GuiMessage::SetAudioOutputSampleRate(sample_rate) => {
                self.set_audio_output_sample_rate(sample_rate);
            }
            GuiMessage::NormalizeSelectedSamples => self.normalize_selected_samples(),
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
            GuiMessage::ClearExtractionHistory => {
                let started_at = Instant::now();
                self.waveform.clear_extraction_history();
                emit_gui_action(
                    "waveform.extraction_history.clear",
                    Some("waveform"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            GuiMessage::NavigateBrowser { delta, extend } => {
                let started_at = Instant::now();
                if let Some(path) = self.folder_browser.navigate_vertical(delta, extend) {
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
                        Some(if delta < 0 { "previous" } else { "next" }),
                        "selected",
                        started_at,
                        None,
                    );
                    self.select_sample(path, context);
                } else {
                    emit_gui_action(
                        "folder_browser.navigate",
                        Some("browser"),
                        Some(if delta < 0 { "previous" } else { "next" }),
                        "edge",
                        started_at,
                        None,
                    );
                }
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
                if self.audio_settings_open {
                    if let Some(action) = action {
                        emit_gui_action(
                            action,
                            Some("waveform"),
                            None,
                            "ignored",
                            started_at,
                            Some("audio_settings_open"),
                        );
                    }
                    return;
                }
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
            GuiMessage::Frame => {
                self.waveform.apply_interaction(WaveformInteraction::Frame);
                self.refresh_playback_progress();
                if self.folder_progress.is_some() {
                    self.progress_tick = (self.progress_tick + 0.035) % 1.0;
                }
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

    fn next_sample_task_id(&mut self) -> u64 {
        let task_id = self.next_sample_task_id;
        self.next_sample_task_id = self.next_sample_task_id.saturating_add(1);
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
        if !confirm_folder_delete(&target) {
            self.sample_status = format!("Delete cancelled for {}", target.name);
            emit_gui_action(
                "folder_browser.delete_selected",
                Some("folder_browser"),
                Some(&target.name),
                "cancelled",
                started_at,
                None,
            );
            return;
        }
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
        if !confirm_file_delete(&target) {
            self.sample_status = format!("Delete cancelled for {}", target.label());
            emit_gui_action(
                "browser.delete_selected_files",
                Some("browser"),
                Some(&target.label()),
                "cancelled",
                started_at,
                None,
            );
            return;
        }

        let loaded_path = self.waveform.path();
        let deleting_loaded_sample = target.paths.iter().any(|path| path == &loaded_path);
        if deleting_loaded_sample {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform = WaveformState::empty();
            self.current_playback_span = None;
        }

        match self.folder_browser.delete_selected_files() {
            Ok(status) => {
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
                self.waveform.record_current_play_selection_extracted();
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
                self.arm_browser_external_drag(context);
            }
            DragHandleMessage::Moved { position } => {
                self.folder_browser.update_drag_pointer(position);
            }
            DragHandleMessage::Ended { .. } => {
                self.folder_browser.clear_drag();
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
        self.folder_browser
            .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
        if started {
            self.arm_browser_external_drag(context);
        } else if ended {
            context.end_external_drag();
        }
    }

    fn arm_browser_external_drag(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let Some(request) = self.folder_browser.external_drag_request() else {
            return;
        };
        context.begin_external_drag(request, GuiMessage::ExternalDragCompleted);
    }

    fn external_drag_completed(&mut self, result: Result<ui::ExternalDragOutcome, String>) {
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
        let task_id = self.next_sample_task_id();
        self.pending_sample_task_id = Some(task_id);
        self.sample_status = format!("Loading {}", sample_path_label(path.as_str()));
        let label = sample_path_label(path.as_str());
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
        context.spawn(
            "gui-sample-load",
            move || {
                let result = WaveformState::load_path(PathBuf::from(&path));
                SampleLoadResult {
                    task_id,
                    path,
                    result,
                }
            },
            GuiMessage::SampleLoadFinished,
        );
    }

    fn finish_sample_load(&mut self, load: SampleLoadResult) {
        let started_at = Instant::now();
        let label = sample_path_label(load.path.as_str());
        if self.pending_sample_task_id != Some(load.task_id) {
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
        self.pending_sample_task_id = None;
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

    fn play_selected_sample(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id()
            && PathBuf::from(path) != self.waveform.path()
        {
            let label = sample_path_label(path);
            emit_gui_action(
                "playback.play_selected_sample",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.select_sample(path.to_string(), context);
            return;
        }
        let (start, end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
        let started_at = Instant::now();
        match self.start_playback_current_span(start_ratio, 1.0) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status =
                    format!("Playing {} from {:.1}%", file_name, start_ratio * 100.0);
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn stop_playback(&mut self) {
        let started_at = Instant::now();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.waveform.stop_playback();
        self.current_playback_span = None;
        let file_name = self.waveform.file_name();
        self.sample_status = format!("Stopped {file_name}");
        emit_gui_action(
            "playback.stop",
            Some("transport"),
            Some(&file_name),
            "success",
            started_at,
            None,
        );
    }

    fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_span(start_ratio, end_ratio, None)
    }

    fn start_playback_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Result<(), String> {
        if self.audio_player.is_none() {
            self.open_configured_audio_player()?;
        }
        if !self.waveform.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let playback_span = self.resolve_playback_span(start_ratio, end_ratio, loop_offset_ratio);
        let start_ratio = playback_span.start_ratio;
        let end_ratio = playback_span.end_ratio;
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        player.set_audio(self.waveform.audio_bytes(), duration);
        player.set_edit_fade_state(self.waveform.edit_selection());
        let playback_start = if self.loop_playback {
            player.play_looped_range_from(
                f64::from(start_ratio),
                f64::from(end_ratio),
                f64::from(playback_span.offset_ratio),
            )?;
            playback_span.offset_ratio
        } else {
            player.play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
            start_ratio
        };
        self.waveform.start_playback(playback_start);
        self.current_playback_span = Some((start_ratio, end_ratio));
        Ok(())
    }

    fn resolve_playback_span(
        &self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> ResolvedPlaybackSpan {
        let requested_start = start_ratio.clamp(0.0, 1.0);
        let requested_end = end_ratio.clamp(requested_start, 1.0);
        if !self.loop_playback {
            return ResolvedPlaybackSpan {
                start_ratio: requested_start,
                end_ratio: requested_end,
                offset_ratio: requested_start,
            };
        }

        let (loop_start, loop_end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        let start_ratio = loop_start.clamp(0.0, 1.0);
        let end_ratio = loop_end.clamp(start_ratio, 1.0);
        let requested_offset = loop_offset_ratio.unwrap_or(requested_start).clamp(0.0, 1.0);
        let offset_ratio = if (start_ratio..=end_ratio).contains(&requested_offset) {
            requested_offset
        } else {
            start_ratio
        };

        ResolvedPlaybackSpan {
            start_ratio,
            end_ratio,
            offset_ratio,
        }
    }

    fn set_volume(&mut self, volume: f32) {
        let started_at = Instant::now();
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.audio_player.as_mut() {
            player.set_volume(self.volume);
        }
        emit_gui_action(
            "playback.volume.set",
            Some("transport"),
            None,
            "success",
            started_at,
            None,
        );
    }

    fn toggle_audio_settings(&mut self) {
        let started_at = Instant::now();
        self.audio_settings_open = !self.audio_settings_open;
        emit_gui_action(
            "audio.settings.toggle",
            Some("top_bar"),
            None,
            if self.audio_settings_open {
                "opened"
            } else {
                "closed"
            },
            started_at,
            None,
        );
    }

    fn set_audio_output_host(&mut self, host: Option<String>) {
        let started_at = Instant::now();
        self.audio_output_config.host = host;
        self.audio_output_config.device = None;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.host.set");
    }

    fn set_audio_output_device(&mut self, device: Option<String>) {
        let started_at = Instant::now();
        self.audio_output_config.device = device;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.device.set");
    }

    fn set_audio_output_sample_rate(&mut self, sample_rate: Option<u32>) {
        let started_at = Instant::now();
        self.audio_output_config.sample_rate = sample_rate;
        self.apply_audio_output_config_change(started_at, "audio.output.sample_rate.set");
    }

    fn apply_audio_output_config_change(&mut self, started_at: Instant, action: &'static str) {
        let restart_span = self
            .waveform
            .is_playing()
            .then_some(self.current_playback_span)
            .flatten();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.audio_player = None;
        self.audio_output_resolved = None;
        self.refresh_audio_options();

        let mut outcome = "success";
        let mut error = None;
        match self.open_configured_audio_player() {
            Ok(()) => {
                if let Some((start, end)) = restart_span {
                    if let Err(err) = self.start_playback_current_span(start, end) {
                        self.waveform.stop_playback();
                        self.current_playback_span = None;
                        self.sample_status =
                            format!("Audio output changed | playback failed: {err}");
                        outcome = "playback_error";
                        error = Some(err);
                    } else {
                        self.sample_status = format!(
                            "Audio output changed | {}",
                            self.audio_engine_detail_label()
                        );
                    }
                } else {
                    self.waveform.stop_playback();
                    self.current_playback_span = None;
                    self.sample_status = format!(
                        "Audio output changed | {}",
                        self.audio_engine_detail_label()
                    );
                }
            }
            Err(err) => {
                self.waveform.stop_playback();
                self.current_playback_span = None;
                self.audio_settings_error = Some(err.clone());
                self.sample_status = format!("Audio output unavailable: {err}");
                outcome = "error";
                error = Some(err);
            }
        }
        emit_gui_action(
            action,
            Some("audio_settings"),
            None,
            outcome,
            started_at,
            error.as_deref(),
        );
    }

    fn open_configured_audio_player(&mut self) -> Result<(), String> {
        let mut player = AudioPlayer::from_config(&self.audio_output_config)?;
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        self.audio_settings_error = None;
        self.audio_player = Some(player);
        Ok(())
    }

    fn refresh_audio_options(&mut self) {
        let mut error = None;
        self.audio_hosts = available_hosts();
        let host_id = self.selected_audio_host_id();
        self.audio_devices = host_id
            .as_deref()
            .and_then(|host_id| match available_devices(host_id) {
                Ok(devices) => Some(devices),
                Err(err) => {
                    error = Some(err.to_string());
                    None
                }
            })
            .unwrap_or_default();
        let device_name = self.selected_audio_device_name();
        self.audio_sample_rates = match (host_id.as_deref(), device_name.as_deref()) {
            (Some(host_id), Some(device_name)) => {
                match supported_sample_rates(host_id, device_name) {
                    Ok(rates) => rates,
                    Err(err) => {
                        error = Some(err.to_string());
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        };
        if error.is_some() {
            self.audio_settings_error = error;
        }
    }

    fn selected_audio_host_id(&self) -> Option<String> {
        self.audio_output_config.host.clone().or_else(|| {
            self.audio_hosts
                .iter()
                .find(|host| host.is_default)
                .or_else(|| self.audio_hosts.first())
                .map(|host| host.id.clone())
        })
    }

    fn selected_audio_device_name(&self) -> Option<String> {
        self.audio_output_config.device.clone().or_else(|| {
            self.audio_devices
                .iter()
                .find(|device| device.is_default)
                .or_else(|| self.audio_devices.first())
                .map(|device| device.name.clone())
        })
    }

    fn audio_engine_pill_label(&self) -> String {
        self.audio_output_resolved
            .as_ref()
            .map(|output| format_sample_rate_label(output.sample_rate))
            .or_else(|| {
                self.audio_output_config
                    .sample_rate
                    .map(format_sample_rate_label)
            })
            .unwrap_or_else(|| {
                if self.audio_settings_error.is_some() {
                    String::from("Audio Err")
                } else {
                    String::from("Audio")
                }
            })
    }

    fn audio_engine_detail_label(&self) -> String {
        self.audio_output_resolved
            .as_ref()
            .map(|output| {
                format!(
                    "{} | {} | {}",
                    self.audio_host_label(output.host_id.as_str()),
                    output.device_name,
                    format_sample_rate_label(output.sample_rate)
                )
            })
            .or_else(|| self.audio_settings_error.clone())
            .unwrap_or_else(|| String::from("Audio output idle"))
    }

    fn audio_host_label(&self, id: &str) -> String {
        self.audio_hosts
            .iter()
            .find(|host| host.id == id)
            .map(|host| host.label.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn toggle_loop_playback(&mut self) {
        let started_at = Instant::now();
        self.loop_playback = !self.loop_playback;
        let mut outcome = "success";
        let mut error = None;
        if self.waveform.is_playing()
            && let Some((start, end)) = self.current_playback_span
        {
            let current = self.current_audio_progress_ratio().unwrap_or(start);
            let result = if self.loop_playback {
                self.start_playback_span(start, end, Some(current))
            } else {
                self.start_playback_current_span(current.clamp(start, end), end)
            };
            if let Err(err) = result {
                self.loop_playback = false;
                self.sample_status = format!("Loop toggle failed: {err}");
                outcome = "error";
                error = Some(err);
            }
        }
        if outcome == "success" {
            self.sample_status = if self.loop_playback {
                String::from("Loop playback enabled")
            } else {
                String::from("Loop playback disabled")
            };
        }
        emit_gui_action(
            "playback.loop.toggle",
            Some("transport"),
            None,
            outcome,
            started_at,
            error.as_deref(),
        );
    }

    fn current_audio_progress_ratio(&self) -> Option<f32> {
        self.audio_player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or_else(|| self.waveform.playhead_ratio())
    }

    fn recover_loop_playback(&mut self, reason: &'static str) -> Result<(), String> {
        let Some((start, end)) = self.current_playback_span else {
            return Err(String::from("No active playback span to loop"));
        };
        let offset = self.current_audio_progress_ratio().unwrap_or(start);
        self.start_playback_span(start, end, Some(offset))?;
        emit_gui_action(
            "playback.loop.recover",
            Some("transport"),
            None,
            reason,
            Instant::now(),
            None,
        );
        Ok(())
    }

    fn retarget_loop_playback_to_play_selection(&mut self) {
        if !self.loop_playback || !self.waveform.is_playing() {
            return;
        }
        let Some(selection) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        if playback_span_matches_selection(self.current_playback_span, selection) {
            return;
        }

        let started_at = Instant::now();
        let current = self
            .current_audio_progress_ratio()
            .unwrap_or_else(|| selection.start());
        let offset = loop_retarget_offset_for_selection(current, selection);
        match self.start_playback_span(selection.start(), selection.end(), Some(offset)) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Loop range updated | {file_name}");
                emit_gui_action(
                    "playback.loop.retarget",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Loop retarget failed: {err}");
                emit_gui_action(
                    "playback.loop.retarget",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio_player.as_ref() {
            player.set_edit_fade_state(self.waveform.edit_selection());
        }
    }

    fn refresh_playback_progress(&mut self) {
        let Some(player) = self.audio_player.as_mut() else {
            return;
        };
        if let Some(error) = player.take_error() {
            let started_at = Instant::now();
            self.waveform.stop_playback();
            self.sample_status = format!("Playback stopped: {error}");
            emit_gui_action(
                "playback.progress",
                Some("transport"),
                None,
                "error",
                started_at,
                Some(&error),
            );
            return;
        }

        let active = player.is_playing();
        let elapsed = player.playback_elapsed();
        let player_looping = player.is_looping();
        let progress = player.progress();
        let should_be_looping = self.loop_playback && self.waveform.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);

        if should_be_looping && (!player_looping || (!active && !within_start_grace)) {
            let reason = if !player_looping {
                "player_not_looping"
            } else {
                "loop_source_inactive"
            };
            if let Err(err) = self.recover_loop_playback(reason) {
                self.loop_playback = false;
                self.waveform.stop_playback();
                self.current_playback_span = None;
                self.sample_status = format!("Loop playback stopped: {err}");
                emit_gui_action(
                    "playback.loop.recover",
                    Some("transport"),
                    None,
                    "error",
                    Instant::now(),
                    Some(&err),
                );
            }
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
            let started_at = Instant::now();
            self.waveform.stop_playback();
            self.current_playback_span = None;
            emit_gui_action(
                "playback.progress",
                Some("transport"),
                None,
                "completed",
                started_at,
                None,
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedPlaybackSpan {
    start_ratio: f32,
    end_ratio: f32,
    offset_ratio: f32,
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

fn loop_retarget_offset_for_selection(
    playhead: f32,
    selection: wavecrate::selection::SelectionRange,
) -> f32 {
    let start = selection.start();
    let end = selection.end();
    if (start..=end).contains(&playhead) {
        playhead
    } else {
        start
    }
}

fn playback_span_matches_selection(
    span: Option<(f32, f32)>,
    selection: wavecrate::selection::SelectionRange,
) -> bool {
    let Some((start, end)) = span else {
        return false;
    };
    (start - selection.start()).abs() <= 0.000_1 && (end - selection.end()).abs() <= 0.000_1
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
            })
            .on_frame(|| GuiMessage::Frame)
            .subscriptions(GuiAppState::worker_subscription)
            .on_scroll(|state, update, _context| {
                if update.node_id == SAMPLE_BROWSER_LIST_ID {
                    state.folder_browser.set_file_view_start_from_scroll_offset(
                        update.offset.y,
                        SAMPLE_BROWSER_ROW_HEIGHT,
                    );
                }
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

fn sample_path_label(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn normalize_wav_file_in_place(path: &Path) -> Result<(), String> {
    ensure_normalizable_wav(path)?;
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid wav: {err}"))?;
    let spec = reader.spec();
    let mut samples = read_wav_samples_as_f32(&mut reader, spec)?;
    if samples.is_empty() {
        return Err(String::from("No audio data to normalize"));
    }
    normalize_peak_in_place(&mut samples);
    let target_spec = hound::WavSpec {
        channels: spec.channels.max(1),
        sample_rate: spec.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    write_f32_wav(path, &samples, target_spec)
}

fn ensure_normalizable_wav(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
    {
        return Ok(());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_else(|| String::from("this file type"));
    Err(format!(
        "Normalize overwrite only supports WAV files; {extension} is not supported"
    ))
}

fn read_wav_samples_as_f32<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
) -> Result<Vec<f32>, String> {
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|err| format!("Sample error: {err}")))
            .collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|err| format!("Sample error: {err}"))
                })
                .collect()
        }
    }
}

fn normalize_peak_in_place(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    if !peak.is_finite() || peak <= f32::EPSILON {
        return;
    }
    let gain = 1.0 / peak;
    for sample in samples {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

fn write_f32_wav(path: &Path, samples: &[f32], spec: hound::WavSpec) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|err| format!("Failed to create file: {err}"))?;
    let buf_writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
    let mut writer = hound::WavWriter::new(buf_writer, spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("Failed to write sample: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
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
        bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if let Some(preview) = state.folder_browser.drag_preview() {
        layers.push(folder_drag_preview_overlay(preview));
    }
    if state.audio_settings_open {
        layers.push(audio_settings_popover(state));
    }
    if layers.len() > 1 {
        ui::stack(layers).fill()
    } else {
        layers.pop().expect("view should contain base content")
    }
}

fn folder_drag_preview_overlay(preview: folder_browser::FolderDragPreview) -> ui::View<GuiMessage> {
    let width =
        (preview.label.chars().count() as f32 * 7.0 + 118.0).clamp(150.0, DRAG_PREVIEW_MAX_WIDTH);
    ui::drag_preview_sized(
        preview.label,
        preview.pointer,
        Vector2::new(width, DRAG_PREVIEW_HEIGHT),
    )
    .key("folder-browser-drag-preview")
}

fn default_gui_shortcut_resolution(
    state: &GuiAppState,
    press: ui::KeyPress,
) -> ui::ShortcutResolution<GuiMessage> {
    if state.folder_browser.rename_active() {
        ui::ShortcutResolution::unhandled()
    } else if state.audio_settings_open {
        if press == ui::KeyPress::new(ui::KeyCode::Escape) {
            ui::ShortcutResolution::action(GuiMessage::CloseAudioSettings)
        } else {
            ui::ShortcutResolution::handled()
        }
    } else if press == ui::KeyPress::new(ui::KeyCode::Escape) {
        ui::ShortcutResolution::action(GuiMessage::StopPlayback)
    } else if press == ui::KeyPress::new(ui::KeyCode::F2) {
        ui::ShortcutResolution::action(GuiMessage::FolderBrowser(
            FolderBrowserMessage::BeginRenameSelected,
        ))
    } else if press == ui::KeyPress::new(ui::KeyCode::Delete) {
        ui::ShortcutResolution::action(GuiMessage::DeleteSelectedItem)
    } else if press == ui::KeyPress::new(ui::KeyCode::E) {
        ui::ShortcutResolution::action(GuiMessage::ExtractPlaymarkedRange)
    } else if press == ui::KeyPress::new(ui::KeyCode::N) {
        if state.folder_browser.selected_file_id().is_some() {
            ui::ShortcutResolution::action(GuiMessage::NormalizeSelectedSamples)
        } else {
            ui::ShortcutResolution::action(GuiMessage::FolderBrowser(
                FolderBrowserMessage::BeginCreateSubfolder,
            ))
        }
    } else if press == ui::KeyPress::new(ui::KeyCode::Space) {
        ui::ShortcutResolution::action(GuiMessage::PlaySelectedSample)
    } else if press == ui::KeyPress::with_command(ui::KeyCode::A) {
        ui::ShortcutResolution::action(GuiMessage::SelectAllSamples)
    } else if press.key == ui::KeyCode::ArrowUp {
        ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
            delta: -1,
            extend: press.shift,
        })
    } else if press.key == ui::KeyCode::ArrowDown {
        ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: press.shift,
        })
    } else if press == ui::KeyPress::new(ui::KeyCode::ArrowLeft) {
        ui::ShortcutResolution::action(GuiMessage::CollapseSelectedFolder)
    } else if press == ui::KeyPress::new(ui::KeyCode::ArrowRight) {
        ui::ShortcutResolution::action(GuiMessage::ExpandSelectedFolder)
    } else {
        ui::ShortcutResolution::unhandled()
    }
}

fn top_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        volume_slider(state.volume),
        ui::spacer().height(20.0).fill_width(),
        audio_engine_pill(state.audio_engine_pill_label(), state.audio_settings_open),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn audio_engine_pill(label: String, active: bool) -> ui::View<GuiMessage> {
    audio_engine_pill_with_id(label, active, AUDIO_ENGINE_PILL_ID, "top-audio-engine-pill")
}

fn audio_engine_pill_with_id(
    label: String,
    active: bool,
    id: u64,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::custom_widget(AudioEnginePill::new(label, active), |output| {
        output.typed_ref::<GuiMessage>().cloned()
    })
    .id(id)
    .key(key)
    .size(AUDIO_ENGINE_PILL_WIDTH, AUDIO_ENGINE_PILL_HEIGHT)
}

fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    ui::custom_widget(VolumeSlider::new(volume), |output| {
        output
            .typed_ref::<VolumeSliderMessage>()
            .copied()
            .map(|message| GuiMessage::SetVolume(message.volume))
    })
    .id(VOLUME_SLIDER_ID)
    .key("top-volume-slider")
    .size(VOLUME_SLIDER_WIDTH, VOLUME_SLIDER_HEIGHT)
}

fn audio_settings_popover(state: &GuiAppState) -> ui::View<GuiMessage> {
    let panel = ui::column(audio_settings_panel_rows(state))
        .key("audio-settings-panel")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Strong,
        })
        .spacing(7.0)
        .padding(8.0)
        .width(AUDIO_SETTINGS_POPUP_WIDTH)
        .height(AUDIO_SETTINGS_POPUP_HEIGHT);
    let centered_panel = ui::column(vec![
        ui::spacer().fill_height(),
        ui::row(vec![
            ui::spacer().height(1.0).fill_width(),
            panel,
            ui::spacer().height(1.0).fill_width(),
        ])
        .fill_width()
        .height(AUDIO_SETTINGS_POPUP_HEIGHT),
        ui::spacer().fill_height(),
    ])
    .fill();
    ui::stack(vec![
        audio_settings_modal_blocker(),
        centered_panel,
        audio_settings_modal_pill(state.audio_engine_pill_label()),
    ])
    .fill()
}

fn audio_settings_modal_blocker() -> ui::View<GuiMessage> {
    ui::custom_widget(AudioSettingsModalBlocker::new(), |_| None)
        .id(AUDIO_SETTINGS_MODAL_BLOCKER_ID)
        .key("audio-settings-modal-blocker")
        .fill()
}

fn audio_settings_modal_pill(label: String) -> ui::View<GuiMessage> {
    ui::row(vec![
        ui::spacer().height(1.0).fill_width(),
        audio_engine_pill_with_id(
            label,
            true,
            AUDIO_SETTINGS_MODAL_PILL_ID,
            "audio-settings-modal-pill",
        ),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn audio_settings_panel_rows(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut rows = vec![
        ui::row(vec![
            ui::text("Audio Engine").height(20.0).fill_width(),
            ui::button("x")
                .subtle()
                .message(GuiMessage::CloseAudioSettings)
                .width(24.0)
                .height(20.0),
        ])
        .fill_width()
        .height(22.0),
        ui::text(state.audio_engine_detail_label())
            .key("audio-settings-detail")
            .fill_width()
            .height(20.0)
            .truncate(),
    ];
    if let Some(error) = state.audio_settings_error.as_ref() {
        rows.push(
            ui::text(error.clone())
                .key("audio-settings-error")
                .style(ui::WidgetStyle {
                    tone: ui::WidgetTone::Danger,
                    prominence: ui::WidgetProminence::Subtle,
                })
                .fill_width()
                .height(20.0)
                .truncate(),
        );
    }
    rows.push(audio_settings_section(
        "Backend",
        audio_host_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Output",
        audio_device_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Sample Rate",
        audio_sample_rate_option_buttons(state),
        4,
    ));
    rows
}

fn audio_settings_section(
    label: &'static str,
    options: Vec<ui::View<GuiMessage>>,
    columns: usize,
) -> ui::View<GuiMessage> {
    let grid_height = audio_option_grid_height(options.len(), columns);
    let mut rows = vec![
        ui::text(label)
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Subtle,
            })
            .fill_width()
            .height(18.0),
    ];
    if options.is_empty() {
        rows.push(ui::text("Unavailable").fill_width().height(20.0));
    } else {
        rows.push(
            ui::grid(options, columns.max(1))
                .fill_width()
                .height(grid_height),
        );
    }
    ui::column(rows)
        .spacing(3.0)
        .fill_width()
        .height(21.0 + grid_height)
}

fn audio_host_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "System default".to_string(),
        state.audio_output_config.host.is_none(),
        GuiMessage::SetAudioOutputHost(None),
    )];
    buttons.extend(state.audio_hosts.iter().map(|host| {
        audio_option_button(
            default_option_label(host.label.as_str(), host.is_default),
            state.audio_output_config.host.as_deref() == Some(host.id.as_str()),
            GuiMessage::SetAudioOutputHost(Some(host.id.clone())),
        )
    }));
    buttons
}

fn audio_device_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Host default".to_string(),
        state.audio_output_config.device.is_none(),
        GuiMessage::SetAudioOutputDevice(None),
    )];
    buttons.extend(state.audio_devices.iter().map(|device| {
        audio_option_button(
            default_option_label(device.name.as_str(), device.is_default),
            state.audio_output_config.device.as_deref() == Some(device.name.as_str()),
            GuiMessage::SetAudioOutputDevice(Some(device.name.clone())),
        )
    }));
    buttons
}

fn audio_sample_rate_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Device default".to_string(),
        state.audio_output_config.sample_rate.is_none(),
        GuiMessage::SetAudioOutputSampleRate(None),
    )];
    buttons.extend(state.audio_sample_rates.iter().copied().map(|rate| {
        audio_option_button(
            format_sample_rate_label(rate),
            state.audio_output_config.sample_rate == Some(rate),
            GuiMessage::SetAudioOutputSampleRate(Some(rate)),
        )
    }));
    buttons
}

fn audio_option_button(label: String, selected: bool, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::button(label)
        .style(ui::WidgetStyle {
            tone: if selected {
                ui::WidgetTone::Accent
            } else {
                ui::WidgetTone::Neutral
            },
            prominence: if selected {
                ui::WidgetProminence::Strong
            } else {
                ui::WidgetProminence::Subtle
            },
        })
        .message(message)
        .fill_width()
        .height(20.0)
}

fn default_option_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (default)")
    } else {
        label.to_string()
    }
}

fn audio_option_grid_height(option_count: usize, columns: usize) -> f32 {
    let columns = columns.max(1);
    let rows = option_count.max(1).div_ceil(columns);
    rows as f32 * 20.0 + rows.saturating_sub(1) as f32 * 4.0
}

fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
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
    ui::custom_widget(ToolbarIconButton::new(icon, enabled, active), |output| {
        output.typed_ref::<GuiMessage>().cloned()
    })
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
            Self::Loop => include_str!(
                "app_core/native_shell/composition/assets/icons/waveform_toolbar/loop.svg"
            ),
            Self::Play => include_str!(
                "app_core/native_shell/composition/assets/icons/waveform_toolbar/play.svg"
            ),
            Self::Stop => include_str!(
                "app_core/native_shell/composition/assets/icons/waveform_toolbar/stop.svg"
            ),
        }
    }
}

#[derive(Clone, Debug)]
struct ToolbarIconButton {
    common: WidgetCommon,
    icon: ToolbarIcon,
}

impl ToolbarIconButton {
    fn new(icon: ToolbarIcon, enabled: bool, active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(28.0, 24.0)));
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.state.disabled = !enabled;
        common.state.active = active;
        Self { common, icon }
    }
}

impl Widget for ToolbarIconButton {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if self.common.state.disabled {
            self.common.state.pressed = false;
            return None;
        }
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(toolbar_button_message(self.icon)))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                }
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => match key {
                radiant::widgets::WidgetKey::Enter | radiant::widgets::WidgetKey::Space => {
                    Some(WidgetOutput::typed(toolbar_button_message(self.icon)))
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let side = bounds.width().min(bounds.height()).min(16.0).max(8.0);
        let icon_rect = Rect::from_min_size(
            Point::new(
                bounds.min.x + (bounds.width() - side) * 0.5,
                bounds.min.y + (bounds.height() - side) * 0.5,
            ),
            Vector2::new(side, side),
        );
        if let Some(image) = rasterize_toolbar_icon(
            self.icon,
            side.round() as usize,
            toolbar_icon_color(theme, self.common.state.disabled, self.common.state.active),
        ) {
            primitives.push(PaintPrimitive::Image(PaintImage {
                widget_id: self.common.id,
                source_rect: None,
                rect: icon_rect,
                image: Arc::new(image),
            }));
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

#[derive(Clone, Debug)]
struct AudioEnginePill {
    common: WidgetCommon,
    label: String,
}

impl AudioEnginePill {
    fn new(label: String, active: bool) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(
                AUDIO_ENGINE_PILL_WIDTH,
                AUDIO_ENGINE_PILL_HEIGHT,
            )),
        );
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_state_layers = false;
        common.state.active = active;
        Self { common, label }
    }
}

impl Widget for AudioEnginePill {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(GuiMessage::ToggleAudioSettings))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                }
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => match key {
                radiant::widgets::WidgetKey::Enter | radiant::widgets::WidgetKey::Space => {
                    Some(WidgetOutput::typed(GuiMessage::ToggleAudioSettings))
                }
                _ => None,
            },
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let hovered_or_pressed = self.common.state.hovered || self.common.state.pressed;
        let fill = if self.common.state.active {
            Rgba8 {
                r: 50,
                g: 54,
                b: 56,
                a: 245,
            }
        } else if hovered_or_pressed {
            Rgba8 {
                r: 42,
                g: 43,
                b: 44,
                a: 245,
            }
        } else {
            Rgba8 {
                r: 31,
                g: 32,
                b: 33,
                a: 235,
            }
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: fill,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
            ),
            color: Rgba8 {
                r: 78,
                g: 79,
                b: 80,
                a: if hovered_or_pressed { 230 } else { 165 },
            },
            width: 1.0,
        }));
        if self.common.state.focused {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
                    Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
                ),
                color: Rgba8 {
                    r: 255,
                    g: 112,
                    b: 86,
                    a: 190,
                },
                width: 1.0,
            }));
        }
        let font_size = 9.0;
        let text_rect = Rect::from_min_max(
            Point::new(bounds.min.x + 5.0, bounds.min.y),
            Point::new(bounds.max.x - 5.0, bounds.max.y),
        );
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: PaintText::from(self.label.as_str()),
            rect: text_rect,
            font_size,
            baseline: Some(((text_rect.height() - font_size) * 0.5 + font_size * 0.78).round()),
            color: Rgba8 {
                r: 183,
                g: 184,
                b: 184,
                a: 235,
            },
            align: PaintTextAlign::Center,
            wrap: TextWrap::None,
        }));
    }
}

#[derive(Clone, Debug)]
struct AudioSettingsModalBlocker {
    common: WidgetCommon,
}

impl AudioSettingsModalBlocker {
    fn new() -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::Pointer;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for AudioSettingsModalBlocker {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn accepts_pointer_move(&self) -> bool {
        false
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct VolumeSliderMessage {
    volume: f32,
}

#[derive(Clone, Debug)]
struct VolumeSlider {
    common: WidgetCommon,
    volume: f32,
}

impl VolumeSlider {
    fn new(volume: f32) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(VOLUME_SLIDER_WIDTH, VOLUME_SLIDER_HEIGHT)),
        );
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            volume: volume.clamp(0.0, 1.0),
        }
    }
}

impl Widget for VolumeSlider {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed {
                    self.volume = volume_from_point(bounds, position);
                    return Some(WidgetOutput::typed(VolumeSliderMessage {
                        volume: self.volume,
                    }));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.volume = volume_from_point(bounds, position);
                Some(WidgetOutput::typed(VolumeSliderMessage {
                    volume: self.volume,
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let was_pressed = self.common.state.pressed;
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                if was_pressed {
                    self.volume = volume_from_point(bounds, position);
                    return Some(WidgetOutput::typed(VolumeSliderMessage {
                        volume: self.volume,
                    }));
                }
                None
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let track_rect = Rect::from_min_max(
            Point::new(bounds.min.x, bounds.min.y + (bounds.height() - 6.0) * 0.5),
            Point::new(bounds.max.x, bounds.min.y + (bounds.height() + 6.0) * 0.5),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: track_rect,
            color: Rgba8 {
                r: 44,
                g: 44,
                b: 44,
                a: 230,
            },
        }));
        let fill_width = (track_rect.width() * self.volume).clamp(
            if self.volume > 0.0 { 1.0 } else { 0.0 },
            track_rect.width(),
        );
        if fill_width > 0.0 {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    track_rect.min,
                    Point::new(track_rect.min.x + fill_width, track_rect.max.y),
                ),
                color: Rgba8 {
                    r: 255,
                    g: 100,
                    b: 76,
                    a: 205,
                },
            }));
        }
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(track_rect.min.x + 0.5, track_rect.min.y + 0.5),
                Point::new(track_rect.max.x - 0.5, track_rect.max.y - 0.5),
            ),
            color: Rgba8 {
                r: 98,
                g: 98,
                b: 98,
                a: if self.common.state.hovered { 230 } else { 170 },
            },
            width: 1.0,
        }));
    }
}

fn volume_from_point(bounds: Rect, position: Point) -> f32 {
    ((position.x - bounds.min.x) / bounds.width().max(1.0)).clamp(0.0, 1.0)
}

fn toolbar_icon_color(theme: &ThemeTokens, disabled: bool, active: bool) -> Rgba8 {
    let mut color = if active {
        theme.highlight_orange
    } else {
        theme.text_primary
    };
    if disabled {
        color.a = (u16::from(color.a) / 2) as u8;
    }
    color
}

fn rasterize_toolbar_icon(icon: ToolbarIcon, side: usize, color: Rgba8) -> Option<ImageRgba> {
    let document = parse_svg_document(icon.svg())?;
    let mut pixels = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
    let sample_offsets = [
        (0.25_f32, 0.25_f32),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];
    for y in 0..side {
        for x in 0..side {
            let mut hits = 0_u8;
            for (offset_x, offset_y) in sample_offsets {
                let world_x = document.view_box_min_x
                    + ((x as f32 + offset_x) / side as f32) * document.view_box_width;
                let world_y = document.view_box_min_y
                    + ((y as f32 + offset_y) / side as f32) * document.view_box_height;
                if point_in_svg_shapes(world_x, world_y, &document.shapes) {
                    hits = hits.saturating_add(1);
                }
            }
            if hits == 0 {
                continue;
            }
            let coverage = hits as f32 / sample_offsets.len() as f32;
            let alpha = ((color.a as f32) * coverage).round().clamp(0.0, 255.0) as u8;
            let index = (y * side + x) * 4;
            pixels[index] = color.r;
            pixels[index + 1] = color.g;
            pixels[index + 2] = color.b;
            pixels[index + 3] = alpha;
        }
    }
    ImageRgba::new(side, side, pixels)
}

fn waveform_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        waveform_panel_header(&state.waveform),
        ui::text(waveform_title(&state.waveform))
            .height(18.0)
            .fill_width()
            .truncate(),
        waveform::waveform_viewport_view(&state.waveform)
            .fill_width()
            .height(WAVEFORM_VIEW_HEIGHT),
        waveform_scrollbar(&state.waveform),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(WAVEFORM_PANEL_HEIGHT)
}

fn waveform_panel_header(waveform: &WaveformState) -> ui::View<GuiMessage> {
    if waveform.has_extraction_history() {
        ui::row([
            ui::text("Waveform").height(18.0).fill_width(),
            ui::button("o")
                .message(GuiMessage::ClearExtractionHistory)
                .key("clear-extraction-history")
                .subtle()
                .size(22.0, 18.0),
        ])
        .fill_width()
        .height(18.0)
        .spacing(4.0)
    } else {
        ui::text("Waveform").height(18.0).fill_width()
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
    let mut scrollbar = radiant::widgets::ScrollbarWidget::new(
        0,
        radiant::widgets::ScrollbarAxis::Horizontal,
        radiant::widgets::WidgetSizing::fixed(radiant::gui::types::Vector2::new(1200.0, 6.0)),
    );
    scrollbar.props.viewport_fraction = waveform.visible_fraction();
    scrollbar.state.offset_fraction = waveform.offset_fraction();
    ui::custom_widget(scrollbar, |output| {
        output
            .typed_ref::<ScrollbarMessage>()
            .copied()
            .map(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => {
                    GuiMessage::Waveform(WaveformInteraction::ScrollTo { offset_fraction })
                }
            })
    })
    .fill_width()
    .height(6.0)
}

fn sample_browser(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let audio_files = state.folder_browser.selected_audio_files();
    let audio_count = audio_files.len();
    let columns = state.folder_browser.visible_file_columns();
    ui::column([
        sample_browser_header(&columns, state.folder_browser.file_sort()),
        sample_browser_rows(&state.folder_browser, &audio_files, &columns),
        sample_browser_status(audio_count),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill()
}

fn sample_browser_header(columns: &[&FileColumn], sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    details_header_row(
        columns
            .iter()
            .map(|column| sample_header_cell(column, sort)),
    )
}

fn sample_header_cell(column: &FileColumn, sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    let marker = if sort.column_id == column.id {
        match sort.direction {
            ui::SortDirection::Ascending => " ^",
            ui::SortDirection::Descending => " v",
        }
    } else {
        ""
    };
    let column_id = column.id.clone();
    let resize_id = column.id.clone();
    ui::row([
        ui::button(format!("{}{marker}", column.label))
            .message(GuiMessage::FolderBrowser(
                FolderBrowserMessage::SortFileColumn(column_id),
            ))
            .key(format!("sample-sort-{}", column.id))
            .align_text(ui::TextAlign::Left)
            .fill_width()
            .height(20.0)
            .input_only(),
        ui::drag_handle()
            .mapped(move |message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFileColumn(
                    resize_id.clone(),
                    message,
                ))
            })
            .key(format!("sample-column-resize-{}", column.id))
            .size(4.0, 20.0),
    ])
    .width(column.width)
    .height(20.0)
    .spacing(1.0)
}

fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
    columns: &[&FileColumn],
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return ui::text("No audio files in selected folder")
            .height(24.0)
            .fill_width()
            .fill_height();
    }

    ui::virtual_list(
        files.iter().copied(),
        |file| {
            sample_browser_row(
                file,
                folder_browser.is_file_selected(&file.id),
                folder_browser.file_rename_view(&file.id),
                columns,
            )
        },
        SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32,
    )
    .id(SAMPLE_BROWSER_LIST_ID)
    .fill()
}

fn sample_browser_row(
    file: &FileEntry,
    selected: bool,
    rename: Option<folder_browser::FileRenameView>,
    columns: &[&FileColumn],
) -> ui::View<GuiMessage> {
    let hit_path = file.id.clone();
    let hit_target =
        ui::custom_widget_mapped(
            SampleFileHitTarget::new(selected),
            move |message| match message {
                SampleFileHitMessage::Activate(modifiers) => {
                    GuiMessage::SelectSampleWithModifiers {
                        path: hit_path.clone(),
                        modifiers,
                    }
                }
                SampleFileHitMessage::Drag(drag) => GuiMessage::DragSampleFile {
                    path: hit_path.clone(),
                    drag,
                },
            },
        )
        .key(format!("sample-row-hit-{}", file.id))
        .fill_width()
        .height(22.0);
    let row = ui::stack([
        hit_target,
        compact_details_row(
            columns
                .iter()
                .map(|column| sample_column_cell(file, rename.clone(), column)),
        ),
    ])
    .key(format!("sample-row-{}", file.id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

fn sample_name_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    width: f32,
) -> ui::View<GuiMessage> {
    let Some(rename) = rename else {
        return sample_file_cell(file, file.stem.clone(), width, "name");
    };
    let mut input = TextInputWidget::new(
        0,
        rename.draft,
        WidgetSizing::fixed(Vector2::new(width, 20.0)),
    );
    input.state.selection_anchor = rename.selection_start;
    input.state.caret = rename.selection_end;
    ui::custom_widget_mapped(input, |message| {
        GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
    })
    .id(rename.input_id)
    .key(format!("sample-rename-input-{}", file.id))
    .width(width)
    .height(20.0)
}

fn sample_column_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    column: &FileColumn,
) -> ui::View<GuiMessage> {
    if column.id == "name" {
        return sample_name_cell(file, rename, column.width);
    }
    sample_file_cell(
        file,
        sample_file_column_value(file, column.id.as_str()),
        column.width,
        column.id.as_str(),
    )
}

fn sample_file_column_value(file: &FileEntry, column_id: &str) -> String {
    match column_id {
        "extension" => file.extension.clone(),
        "size" => file.size.clone(),
        "modified" => file.modified.clone(),
        "kind" => file.kind.clone(),
        "path" => file.id.clone(),
        _ => file.stem.clone(),
    }
}

fn sample_file_cell(
    file: &FileEntry,
    value: String,
    width: f32,
    column_id: &str,
) -> ui::View<GuiMessage> {
    ui::text(value)
        .key(format!("sample-{}-{column_id}", file.id))
        .height(20.0)
        .width(width)
        .truncate()
}

fn compact_details_row(
    children: impl IntoIterator<Item = ui::View<GuiMessage>>,
) -> ui::View<GuiMessage> {
    ui::row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

fn details_header_row(
    children: impl IntoIterator<Item = ui::View<GuiMessage>>,
) -> ui::View<GuiMessage> {
    ui::row(children)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(10.0)
}

fn sample_browser_status(audio_count: usize) -> ui::View<GuiMessage> {
    ui::row([
        ui::text("Listed").height(20.0).width(90.0),
        ui::text(format!(
            "{audio_count} audio file{} in selected folder",
            if audio_count == 1 { "" } else { "s" }
        ))
        .height(20.0)
        .fill_width(),
    ])
    .padding_x(3.0)
    .fill_width()
    .height(28.0)
}

#[derive(Clone, Debug)]
struct SampleFileHitTarget {
    common: WidgetCommon,
    selected: bool,
    dragged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SampleFileHitMessage {
    Activate(PointerModifiers),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    fn new(selected: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected,
            dragged: false,
        }
    }
}

impl Widget for SampleFileHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed {
                    let message = if self.dragged {
                        DragHandleMessage::Moved { position }
                    } else {
                        self.dragged = true;
                        DragHandleMessage::Started { position }
                    };
                    return Some(WidgetOutput::typed(SampleFileHitMessage::Drag(message)));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.dragged = false;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
            } => {
                let activated =
                    self.common.state.pressed && !self.dragged && bounds.contains(position);
                let dragged = self.common.state.pressed && self.dragged;
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.dragged = false;
                if dragged {
                    return Some(WidgetOutput::typed(SampleFileHitMessage::Drag(
                        DragHandleMessage::Ended { position },
                    )));
                }
                activated.then(|| WidgetOutput::typed(SampleFileHitMessage::Activate(modifiers)))
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if self.selected {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 120,
                },
            }));
        }

        if self.common.state.pressed || self.common.state.hovered {
            let alpha = if self.common.state.pressed { 170 } else { 155 };
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: Rgba8 {
                    r: 255,
                    g: 108,
                    b: 88,
                    a: alpha,
                },
            }));
        }

        if !self.selected {
            return;
        }
        let marker_height = (bounds.height() - 8.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_size(
                Point::new(
                    bounds.min.x + 1.0,
                    bounds.min.y + (bounds.height() - marker_height) * 0.5,
                ),
                Vector2::new(3.0, marker_height),
            ),
            color: Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            },
        }));
    }
}

fn bottom_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        ui::text("1 sample").height(20.0).width(120.0),
        ui::text(bottom_status_text(state))
            .height(20.0)
            .fill_width(),
        worker_progress_bar(state),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn bottom_status_text(state: &GuiAppState) -> String {
    state
        .folder_progress
        .as_ref()
        .map(|progress| {
            if progress.total == 0 {
                format!(
                    "{} {} | {} items found",
                    progress.phase, progress.label, progress.completed
                )
            } else {
                format!(
                    "{} {} | {}/{} | {}",
                    progress.phase,
                    progress.label,
                    progress.completed.min(progress.total),
                    progress.total,
                    progress.detail
                )
            }
        })
        .unwrap_or_else(|| state.sample_status.clone())
}

fn worker_progress_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    let Some(progress) = state.folder_progress.as_ref() else {
        return ui::text("").width(0.0).height(10.0);
    };
    if progress.total == 0 {
        return ui::text("").width(0.0).height(10.0);
    }
    let track_width = 180.0;
    let fraction = (progress.completed as f32 / progress.total.max(1) as f32).clamp(0.0, 1.0);
    let filled = (track_width * fraction).clamp(8.0, track_width);
    let empty = (track_width - filled).max(0.0);
    ui::row([
        ui::text("")
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Strong,
            })
            .width(filled)
            .height(8.0),
        ui::text("")
            .style(ui::WidgetStyle::default())
            .width(empty)
            .height(8.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .width(track_width)
    .height(10.0)
}

fn confirm_folder_delete(target: &folder_browser::FolderDeleteTargetView) -> bool {
    if cfg!(test) {
        return true;
    }
    let message = format!(
        "Delete {} and all files inside it?\n\nThis cannot be undone from the default GUI.",
        target.path.display()
    );
    matches!(
        MessageDialog::new()
            .set_title("Delete folder")
            .set_description(message)
            .set_level(MessageLevel::Warning)
            .set_buttons(MessageButtons::YesNo)
            .show(),
        MessageDialogResult::Yes
    )
}

fn confirm_file_delete(target: &folder_browser::FileDeleteTargetView) -> bool {
    if cfg!(test) {
        return true;
    }
    let message = if target.paths.len() == 1 {
        format!(
            "Delete {}?\n\nThis cannot be undone from the default GUI.",
            target.label()
        )
    } else {
        format!(
            "Delete {} selected files?\n\nThis cannot be undone from the default GUI.",
            target.paths.len()
        )
    };
    matches!(
        MessageDialog::new()
            .set_title("Delete file")
            .set_description(message)
            .set_level(MessageLevel::Warning)
            .set_buttons(MessageButtons::YesNo)
            .show(),
        MessageDialogResult::Yes
    )
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
        runtime::{PaintPrimitive, RuntimeBridge, SurfaceRuntime, UiSurface},
        widgets::{DragHandleMessage, PointerButton, PointerModifiers, Widget, WidgetInput},
    };
    use std::{
        ffi::OsString,
        fs,
        sync::{Arc, Mutex, mpsc},
    };

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
            next_sample_task_id: 1,
            pending_sample_task_id: None,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            audio_settings_open: false,
            audio_settings_error: None,
            current_playback_span: None,
        }
    }

    struct StaticSurfaceBridge {
        surface: Arc<UiSurface<super::GuiMessage>>,
    }

    impl RuntimeBridge<super::GuiMessage> for StaticSurfaceBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<super::GuiMessage>> {
            Arc::clone(&self.surface)
        }
    }

    struct CapturingSurfaceBridge {
        surface: Arc<UiSurface<super::GuiMessage>>,
        messages: Arc<Mutex<Vec<super::GuiMessage>>>,
    }

    impl RuntimeBridge<super::GuiMessage> for CapturingSurfaceBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<super::GuiMessage>> {
            Arc::clone(&self.surface)
        }

        fn reduce_message(&mut self, message: super::GuiMessage) {
            self.messages.lock().expect("message capture").push(message);
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
    fn audio_settings_escape_shortcut_closes_modal() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::Escape));

        assert_eq!(
            resolution.action,
            Some(super::GuiMessage::CloseAudioSettings)
        );
        assert!(resolution.handled);
    }

    #[test]
    fn audio_settings_modal_blocks_background_shortcuts() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;

        let resolution =
            super::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::N));

        assert_eq!(resolution.action, None);
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
            next_sample_task_id: 1,
            pending_sample_task_id: None,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            audio_settings_open: false,
            audio_settings_error: None,
            current_playback_span: None,
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
            next_sample_task_id: 1,
            pending_sample_task_id: None,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: super::DEFAULT_VOLUME,
            audio_output_config: super::AudioOutputConfig::default(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            audio_settings_open: false,
            audio_settings_error: None,
            current_playback_span: None,
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
        let task_id = state.pending_sample_task_id.expect("sample load queued");
        state.apply_message(
            super::GuiMessage::SampleLoadFinished(super::SampleLoadResult {
                task_id,
                path: sample_path.clone(),
                result: super::WaveformState::load_path(sample_path.clone().into()),
            }),
            &mut context,
        );

        assert_eq!(
            state.folder_browser.selected_file_id(),
            Some(sample_path.as_str())
        );
        assert_eq!(state.waveform.file_name(), "portal_SS_kick_003.wav");
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
    fn toolbar_icon_assets_parse_and_rasterize() {
        for icon in [
            super::ToolbarIcon::Loop,
            super::ToolbarIcon::Play,
            super::ToolbarIcon::Stop,
        ] {
            assert!(super::parse_svg_document(icon.svg()).is_some());
            let image = super::rasterize_toolbar_icon(
                icon,
                16,
                super::Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            )
            .expect("icon should rasterize");
            assert_eq!(image.width, 16);
            assert_eq!(image.height, 16);
            assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }

    #[test]
    fn toolbar_icon_button_emits_transport_message_on_release() {
        let mut button = super::ToolbarIconButton::new(super::ToolbarIcon::Loop, true, false);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(28.0, 24.0));

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(12.0, 12.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            ),
            None
        );
        let output = button
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(12.0, 12.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("button output");

        assert_eq!(
            output.typed_ref::<super::GuiMessage>(),
            Some(&super::GuiMessage::ToggleLoopPlayback)
        );
    }

    #[test]
    fn toolbar_icon_button_active_state_tints_icon_without_button_chrome() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(28.0, 24.0));
        let theme = radiant::theme::ThemeTokens::default();
        let active = super::ToolbarIconButton::new(super::ToolbarIcon::Loop, true, true);
        let idle = super::ToolbarIconButton::new(super::ToolbarIcon::Loop, true, false);
        let mut active_primitives = Vec::new();
        let mut idle_primitives = Vec::new();

        active.append_paint(&mut active_primitives, bounds, &Default::default(), &theme);
        idle.append_paint(&mut idle_primitives, bounds, &Default::default(), &theme);

        assert_no_button_chrome(&active_primitives);
        assert_eq!(
            first_visible_icon_rgb(&active_primitives),
            toolbar_rgb(theme.highlight_orange)
        );
        assert_eq!(
            first_visible_icon_rgb(&idle_primitives),
            toolbar_rgb(theme.text_primary)
        );
    }

    fn assert_no_button_chrome(primitives: &[PaintPrimitive]) {
        assert!(
            primitives.iter().all(|primitive| !matches!(
                primitive,
                PaintPrimitive::FillRect(_) | PaintPrimitive::StrokeRect(_)
            )),
            "toolbar icon buttons should not paint a fill or border"
        );
    }

    fn first_visible_icon_rgb(primitives: &[PaintPrimitive]) -> (u8, u8, u8) {
        primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Image(image) => image
                    .image
                    .pixels
                    .chunks_exact(4)
                    .find(|pixel| pixel[3] > 0)
                    .map(|pixel| (pixel[0], pixel[1], pixel[2])),
                _ => None,
            })
            .expect("toolbar icon paints visible pixels")
    }

    fn toolbar_rgb(color: super::Rgba8) -> (u8, u8, u8) {
        (color.r, color.g, color.b)
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
        let mut hit_target = super::SampleFileHitTarget::new(false);

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

        let mut refreshed_hit_target = super::SampleFileHitTarget::new(false);
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
            output.typed_ref::<super::SampleFileHitMessage>(),
            Some(&super::SampleFileHitMessage::Activate(PointerModifiers {
                command: true,
                shift: true,
                ..Default::default()
            }))
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
        assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
        assert!(slider_fills >= 2, "expected track and fill rects");
    }

    #[test]
    fn volume_slider_drag_emits_normalized_volume() {
        let mut slider = super::VolumeSlider::new(0.25);
        let bounds = Rect::from_min_size(Point::new(10.0, 0.0), Vector2::new(100.0, 14.0));

        let output = slider
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(85.0, 7.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("volume press should emit");
        assert_eq!(
            output.typed_ref::<super::VolumeSliderMessage>(),
            Some(&super::VolumeSliderMessage { volume: 0.75 })
        );

        let output = slider
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(35.0, 7.0),
                },
            )
            .expect("volume drag should emit");
        assert_eq!(
            output.typed_ref::<super::VolumeSliderMessage>(),
            Some(&super::VolumeSliderMessage { volume: 0.25 })
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
        let mut pill = super::AudioEnginePill::new(String::from("48 kHz"), false);
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
    fn audio_settings_toggle_uses_cached_device_options() {
        let mut state = gui_state_for_span_tests();
        state.audio_hosts = vec![super::AudioHostSummary {
            id: String::from("cached-host"),
            label: String::from("Cached Host"),
            is_default: true,
        }];

        state.toggle_audio_settings();

        assert!(state.audio_settings_open);
        assert_eq!(state.audio_hosts.len(), 1);
        assert_eq!(state.audio_hosts[0].id, "cached-host");
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

        assert!(texts.iter().any(|text| text == "Audio Engine"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Backend"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Output"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
        assert!(
            !texts.iter().any(|text| text.contains("Input")),
            "{texts:?}"
        );
    }

    #[test]
    fn audio_settings_popover_centers_panel_in_window() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_error = None;
        let frame =
            radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
                .frame(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
                    &radiant::theme::ThemeTokens::default(),
                );
        let title_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str() == "Audio Engine" => {
                    Some(text.rect)
                }
                _ => None,
            })
            .expect("audio settings title paints");

        assert!((60.0..=80.0).contains(&title_rect.min.x), "{title_rect:?}");
        assert!((28.0..=40.0).contains(&title_rect.min.y), "{title_rect:?}");
    }

    #[test]
    fn audio_settings_popover_blocks_pointer_input_behind_panel() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;
        let surface = Arc::new(UiSurface::new(super::view(&mut state).into_node()));
        let mut runtime =
            SurfaceRuntime::new(StaticSurfaceBridge { surface }, Vector2::new(960.0, 540.0));
        let waveform_point = Point::new(340.0, 145.0);

        let target = runtime.dispatch_input_at(
            waveform_point,
            WidgetInput::PointerPress {
                position: waveform_point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert!(target.is_some());
        assert_ne!(target, Some(12));
    }

    #[test]
    fn audio_settings_modal_top_pill_toggles_settings_closed() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        state.audio_settings_open = true;
        let surface = Arc::new(UiSurface::new(super::view(&mut state).into_node()));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            CapturingSurfaceBridge {
                surface,
                messages: Arc::clone(&messages),
            },
            Vector2::new(960.0, 540.0),
        );
        let pill_point = Point::new(920.0, 14.0);

        let press_target = runtime.dispatch_input_at(
            pill_point,
            WidgetInput::PointerPress {
                position: pill_point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let release_target = runtime.dispatch_input_at(
            pill_point,
            WidgetInput::PointerRelease {
                position: pill_point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(press_target, Some(super::AUDIO_SETTINGS_MODAL_PILL_ID));
        assert_eq!(release_target, Some(super::AUDIO_SETTINGS_MODAL_PILL_ID));
        assert_eq!(
            messages.lock().expect("message capture").as_slice(),
            &[super::GuiMessage::ToggleAudioSettings]
        );
    }

    #[test]
    fn audio_settings_modal_ignores_waveform_selection_messages() {
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

        assert_eq!(state.waveform.play_mark_ratio(), None);
        assert_eq!(state.waveform.play_selection(), None);
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
        let widget = super::SampleFileHitTarget::new(true);
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
        let mut hit_target = super::SampleFileHitTarget::new(false);

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
    fn waveform_panel_shows_clear_extraction_history_control_only_when_needed() {
        let mut state = GuiAppState::load_default().expect("default state loads");
        let empty_frame =
            radiant::runtime::UiSurface::new(super::waveform_panel(&state).into_node()).frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 240.0)),
                &radiant::theme::ThemeTokens::default(),
            );
        assert!(!frame_has_text(&empty_frame, "o"));

        state
            .waveform
            .apply_interaction(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.2,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.4 });
        state.waveform.record_current_play_selection_extracted();
        let history_frame =
            radiant::runtime::UiSurface::new(super::waveform_panel(&state).into_node()).frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 240.0)),
                &radiant::theme::ThemeTokens::default(),
            );

        assert!(frame_has_text(&history_frame, "o"));
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
