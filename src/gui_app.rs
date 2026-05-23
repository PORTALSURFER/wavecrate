//! Default Wavecrate GUI application built on Radiant's current public API.

use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
    available_devices, available_hosts, supported_sample_rates,
};
use wavecrate::logging;
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

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
mod playback;
mod sample_browser_view;
mod sample_load_actions;
mod shortcuts;
mod status_bar;
mod toolbar;
mod waveform;
mod waveform_panel;
#[cfg(test)]
use audio_settings::audio_settings_popover;
use audio_settings::{format_sample_rate_label, top_status_bar};
use context_menu::BrowserContextMenu;
#[cfg(test)]
use context_menu::BrowserContextTargetKind;
#[cfg(test)]
use file_actions::format_copy_path;
use file_actions::{normalize_wav_file_in_place, sample_path_label};
use folder_browser::{
    FolderBrowserMessage, FolderBrowserState, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanResult,
};
use launch::emit_gui_action;
pub(crate) use launch::run;
#[cfg(test)]
use launch::{DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, debug_layout_requested};
use sample_browser_view::sample_browser;
use sample_load_actions::{NormalizedWaveformReload, WaveformPlaybackResume};
use shortcuts::default_gui_shortcut_resolution;
use toolbar::main_toolbar;
#[cfg(test)]
use toolbar::{ToolbarIcon, toolbar_icon_button};
use waveform::{WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState};
use waveform_panel::waveform_panel;

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

    fn toggle_audio_backend_dropdown(&mut self) {
        self.audio_backend_dropdown_open = !self.audio_backend_dropdown_open;
        self.audio_output_dropdown_open = false;
        self.audio_sample_rate_dropdown_open = false;
    }

    fn toggle_audio_output_dropdown(&mut self) {
        self.audio_output_dropdown_open = !self.audio_output_dropdown_open;
        self.audio_backend_dropdown_open = false;
        self.audio_sample_rate_dropdown_open = false;
    }

    fn toggle_audio_sample_rate_dropdown(&mut self) {
        self.audio_sample_rate_dropdown_open = !self.audio_sample_rate_dropdown_open;
        self.audio_backend_dropdown_open = false;
        self.audio_output_dropdown_open = false;
    }

    fn apply_waveform_message(&mut self, message: WaveformInteraction) {
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

    fn apply_message(&mut self, message: GuiMessage, context: &mut ui::UpdateContext<GuiMessage>) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::FolderBrowser(message) => {
                self.apply_folder_browser_message(message, context);
            }
            GuiMessage::FolderScanProgress(progress) => {
                self.apply_folder_scan_progress(progress);
            }
            GuiMessage::FolderScanDiscoveryBatch(batch) => {
                self.apply_folder_scan_discovery_batch(batch);
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
                self.toggle_audio_backend_dropdown();
            }
            GuiMessage::ToggleAudioOutputDropdown => {
                self.toggle_audio_output_dropdown();
            }
            GuiMessage::ToggleAudioSampleRateDropdown => {
                self.toggle_audio_sample_rate_dropdown();
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
                self.focus_rename_input(input_id, context);
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(),
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(),
            GuiMessage::NavigateBrowser { delta, extend } => {
                self.navigate_browser(delta, extend, context);
            }
            GuiMessage::SelectAllSamples => {
                self.select_all_samples();
            }
            GuiMessage::CollapseSelectedFolder => {
                self.collapse_selected_folder();
            }
            GuiMessage::ExpandSelectedFolder => {
                self.expand_selected_folder();
            }
            GuiMessage::Waveform(message) => {
                self.apply_waveform_message(message);
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
            let playback = was_playing.then_some(WaveformPlaybackResume {
                start_ratio: restart_ratio,
                span: restart_span,
            });
            if let Err(error) = self.reload_normalized_waveform(NormalizedWaveformReload {
                path: &loaded_path,
                playback,
            }) {
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

#[cfg(test)]
mod tests;
