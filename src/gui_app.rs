//! Default Wavecrate GUI application built on Radiant's current public API.

use radiant::gui::types::{ImageRgba, Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{
    NativeRunOptions, NativeTextOptions, PaintFillRect, PaintImage, PaintPrimitive, PaintStrokeRect,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, ScrollbarMessage,
    TextInputWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
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
use wavecrate::audio::AudioPlayer;
use wavecrate::gui::svg::{parse_svg_document, point_in_svg_shapes};
use wavecrate::gui_runtime::wavecrate_ui_font_path;
use wavecrate::logging::{self, ActionDebugEvent, emit_action_debug_event};

mod folder_browser;
mod waveform;
use folder_browser::{
    FileEntry, FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch,
    FolderScanProgress, FolderScanRequest, FolderScanResult,
};
use waveform::{WaveformInteraction, WaveformState};

const DEBUG_LAYOUT_ARG: &str = "--debug-layout";
const DEBUG_LAYOUT_SHORT_ARG: &str = "-debug-layout";
const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;
const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;

#[derive(Clone, Debug, PartialEq)]
enum GuiMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    FolderScanProgress(FolderScanProgress),
    FolderScanDiscoveryBatch(FolderScanDiscoveryBatch),
    FolderScanFinished(FolderScanResult),
    SelectSample(String),
    SampleLoadFinished(SampleLoadResult),
    PlaySelectedSample,
    StopPlayback,
    FocusRenameInput(u64),
    DeleteSelectedFolder,
    ExtractPlaymarkedRange,
    NavigateBrowser(i32),
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
}

impl GuiAppState {
    fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let (worker_sender, worker_receiver) = mpsc::channel();
        let state = Self {
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
        };
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
            GuiMessage::SelectSample(path) => self.select_sample(path, context),
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result),
            GuiMessage::PlaySelectedSample => self.play_selected_sample(context),
            GuiMessage::StopPlayback => self.stop_playback(),
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
            GuiMessage::DeleteSelectedFolder => self.delete_selected_folder(),
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(),
            GuiMessage::NavigateBrowser(delta) => {
                let started_at = Instant::now();
                if let Some(path) = self.folder_browser.navigate_vertical(delta) {
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
                self.waveform.apply_interaction(message);
                self.sync_edit_fade_audio_state();
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
        let started_at = Instant::now();
        self.folder_browser.select_file(path.clone());
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
        if self.audio_player.is_none() {
            self.audio_player = Some(AudioPlayer::new()?);
        }
        if !self.waveform.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let start_ratio = start_ratio.clamp(0.0, 1.0);
        let end_ratio = end_ratio.clamp(start_ratio, 1.0);
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        player.set_audio(self.waveform.audio_bytes(), duration);
        player.set_edit_fade_state(self.waveform.edit_selection());
        player.play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
        self.waveform.start_playback(start_ratio);
        Ok(())
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
        if player.is_playing() {
            if let Some(progress) = player.progress() {
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
            let started_at = Instant::now();
            self.waveform.stop_playback();
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
            .shortcuts(|state, _, press, _| {
                if state.folder_browser.rename_active() {
                    ui::ShortcutResolution::unhandled()
                } else if press == ui::KeyPress::new(ui::KeyCode::F2) {
                    ui::ShortcutResolution::action(GuiMessage::FolderBrowser(
                        FolderBrowserMessage::BeginRenameSelected,
                    ))
                } else if press == ui::KeyPress::new(ui::KeyCode::Delete) {
                    ui::ShortcutResolution::action(GuiMessage::DeleteSelectedFolder)
                } else if press == ui::KeyPress::new(ui::KeyCode::E) {
                    ui::ShortcutResolution::action(GuiMessage::ExtractPlaymarkedRange)
                } else if press == ui::KeyPress::new(ui::KeyCode::Space) {
                    ui::ShortcutResolution::action(GuiMessage::PlaySelectedSample)
                } else if press == ui::KeyPress::new(ui::KeyCode::ArrowUp) {
                    ui::ShortcutResolution::action(GuiMessage::NavigateBrowser(-1))
                } else if press == ui::KeyPress::new(ui::KeyCode::ArrowDown) {
                    ui::ShortcutResolution::action(GuiMessage::NavigateBrowser(1))
                } else if press == ui::KeyPress::new(ui::KeyCode::ArrowLeft) {
                    ui::ShortcutResolution::action(GuiMessage::CollapseSelectedFolder)
                } else if press == ui::KeyPress::new(ui::KeyCode::ArrowRight) {
                    ui::ShortcutResolution::action(GuiMessage::ExpandSelectedFolder)
                } else {
                    ui::ShortcutResolution::unhandled()
                }
            })
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
    ui::column([
        top_status_bar(),
        center_panel(state),
        bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill()
}

fn top_status_bar() -> ui::View<GuiMessage> {
    ui::row([
        ui::text("Wavecrate").height(20.0).width(120.0),
        ui::text("Wavecrate GUI").height(20.0).fill_width(),
        ui::text("ready").height(20.0).width(80.0),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn center_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
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

fn main_area(state: &GuiAppState) -> ui::View<GuiMessage> {
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
    Play,
    Stop,
}

impl ToolbarIcon {
    fn svg(self) -> &'static str {
        match self {
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
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
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
        let tokens = radiant::widgets::resolve_widget_visual_tokens(
            theme,
            self.common.style,
            self.common.state,
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.fill,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.border,
            width: 1.0,
        }));
        if self.common.state.focused && self.common.paint.paints_focus {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
                    Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
                ),
                color: tokens.emphasis,
                width: 1.0,
            }));
        }
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
            toolbar_icon_color(tokens.foreground, self.common.state.disabled),
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
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}

fn toolbar_icon_color(mut color: Rgba8, disabled: bool) -> Rgba8 {
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
        ui::text("Waveform").height(18.0).fill_width(),
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

fn sample_browser(state: &GuiAppState) -> ui::View<GuiMessage> {
    let audio_files = state.folder_browser.selected_audio_files();
    let audio_count = audio_files.len();
    ui::column([
        sample_browser_header(),
        sample_browser_rows(&state.folder_browser, &audio_files),
        sample_browser_status(audio_count),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill()
}

fn sample_browser_header() -> ui::View<GuiMessage> {
    details_header_row([
        sample_header_cell("Name", SAMPLE_NAME_WIDTH),
        sample_header_cell("Ext", SAMPLE_EXT_WIDTH),
        sample_header_cell("Size", SAMPLE_SIZE_WIDTH),
        sample_header_cell("Modified", SAMPLE_MODIFIED_WIDTH),
    ])
}

const SAMPLE_NAME_WIDTH: f32 = 240.0;
const SAMPLE_EXT_WIDTH: f32 = 54.0;
const SAMPLE_SIZE_WIDTH: f32 = 78.0;
const SAMPLE_MODIFIED_WIDTH: f32 = 112.0;

fn sample_header_cell(label: &str, width: f32) -> ui::View<GuiMessage> {
    ui::text(label).height(20.0).width(width)
}

fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return ui::text("No audio files in selected folder")
            .height(24.0)
            .fill_width()
            .fill_height();
    }

    ui::scroll(
        ui::column(
            files
                .iter()
                .map(|file| {
                    sample_browser_row(
                        file,
                        folder_browser.selected_file_id() == Some(file.id.as_str()),
                        folder_browser.file_rename_view(&file.id),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .spacing(1.0)
        .fill_width(),
    )
    .fill()
}

fn sample_browser_row(
    file: &FileEntry,
    selected: bool,
    rename: Option<folder_browser::FileRenameView>,
) -> ui::View<GuiMessage> {
    let hit_path = file.id.clone();
    let hit_target = ui::custom_widget_mapped(SampleFileHitTarget::new(), move |()| {
        GuiMessage::SelectSample(hit_path.clone())
    })
    .key(format!("sample-row-hit-{}", file.id))
    .fill_width()
    .height(22.0);
    let row = ui::stack([
        hit_target,
        compact_details_row([
            sample_name_cell(file, rename),
            sample_file_cell(file, file.extension.clone(), SAMPLE_EXT_WIDTH, "extension"),
            sample_file_cell(file, file.size.clone(), SAMPLE_SIZE_WIDTH, "size"),
            sample_file_cell(
                file,
                file.modified.clone(),
                SAMPLE_MODIFIED_WIDTH,
                "modified",
            ),
        ]),
    ])
    .key(format!("sample-row-{}", file.id))
    .fill_width()
    .height(22.0)
    .hoverable();
    if selected {
        row.style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
    } else {
        row
    }
}

fn sample_name_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
) -> ui::View<GuiMessage> {
    let Some(rename) = rename else {
        return sample_file_cell(file, file.stem.clone(), SAMPLE_NAME_WIDTH, "name");
    };
    let mut input = TextInputWidget::new(
        0,
        rename.draft,
        WidgetSizing::fixed(Vector2::new(SAMPLE_NAME_WIDTH, 20.0)),
    );
    input.state.selection_anchor = rename.selection_start;
    input.state.caret = rename.selection_end;
    ui::custom_widget_mapped(input, |message| {
        GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
    })
    .id(rename.input_id)
    .key(format!("sample-rename-input-{}", file.id))
    .width(SAMPLE_NAME_WIDTH)
    .height(20.0)
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
    pressed: bool,
}

impl SampleFileHitTarget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            pressed: false,
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
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let activated = self.pressed && bounds.contains(position);
                self.pressed = false;
                activated.then(|| WidgetOutput::typed(()))
            }
            _ => None,
        }
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

#[cfg(test)]
mod tests {
    use super::waveform::WaveformSelectionKind;
    use super::{
        DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_FOLDER_WIDTH, GuiAppState,
        MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction, debug_layout_requested,
    };
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        prelude::{self as ui, IntoView},
        runtime::PaintPrimitive,
        widgets::{DragHandleMessage, PointerButton, Widget, WidgetInput},
    };
    use std::{ffi::OsString, sync::mpsc};

    fn selected_asset_file_path(browser: &super::FolderBrowserState, name: &str) -> String {
        browser
            .selected_audio_files()
            .iter()
            .find(|file| file.name == name)
            .unwrap_or_else(|| panic!("expected bundled asset {name} to be visible"))
            .id
            .clone()
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
        };
        let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");

        let mut context = ui::UpdateContext::default();
        state.apply_message(
            super::GuiMessage::SelectSample(sample_path.clone()),
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

        let mut context = ui::UpdateContext::default();
        state.play_selected_sample(&mut context);

        assert!(state.waveform.is_playing());
        assert_eq!(state.waveform.play_mark_ratio(), Some(0.25));
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
    fn toolbar_icon_assets_parse_and_rasterize() {
        for icon in [super::ToolbarIcon::Play, super::ToolbarIcon::Stop] {
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
        let mut button = super::ToolbarIconButton::new(super::ToolbarIcon::Stop, true, false);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(28.0, 24.0));

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(12.0, 12.0),
                    button: PointerButton::Primary,
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
                },
            )
            .expect("button output");

        assert_eq!(
            output.typed_ref::<super::GuiMessage>(),
            Some(&super::GuiMessage::StopPlayback)
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
    fn sample_browser_frame_paints_column_and_file_text() {
        let state = GuiAppState::load_default().expect("default state loads");
        let surface = super::sample_browser(&state).into_node();
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

        assert!(texts.iter().any(|text| text == "Name"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text.starts_with("portal_SS_")),
            "{texts:?}"
        );
        assert!(
            !frame
                .paint_plan
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_))),
            "sample rows should not paint per-cell button chrome"
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
}
