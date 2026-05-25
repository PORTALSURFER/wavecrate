//! Default Wavecrate GUI application built on Radiant's current public API.

use radiant::gui::types::Point;
use radiant::prelude as ui;
use radiant::runtime::NativeFileDrop;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};
use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
    available_devices, available_hosts, supported_sample_rates,
};
use wavecrate::logging;
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
mod playback;
mod sample_browser_view;
mod sample_load_actions;
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
#[cfg(test)]
use sample_browser_view::sample_browser;
use sample_load_actions::{NormalizedWaveformReload, WaveformPlaybackResume};
use shortcuts::default_gui_shortcut_resolution;
#[cfg(test)]
use toolbar::{ToolbarIcon, toolbar_icon_button, toolbar_icon_svg};
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
const AUDIO_ENGINE_PILL_ID: u64 = 31_100;
const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 360.0;
const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 344.0;
const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
const DRAG_PREVIEW_HEIGHT: f32 = 20.0;
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
    native_file_drop_hover: Option<NativeFileDropHover>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeFileDropHover {
    path: PathBuf,
    supported: bool,
}

impl GuiAppState {
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

    fn apply_waveform_message(
        &mut self,
        message: WaveformInteraction,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let action = waveform_interaction_action(&message);
        let active_drag = self.waveform.active_drag_kind();
        if let WaveformInteraction::DragPlaySelectionExport(drag) = message
            && !self.drag_waveform_play_selection(drag, context)
        {
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
            GuiMessage::RemoveContextSource => self.remove_context_source(),
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
            GuiMessage::DropWaveformSelectionOnSampleList => {
                self.drop_waveform_play_selection_on_sample_list(context);
            }
            GuiMessage::Waveform(message) => {
                self.apply_waveform_message(message, context);
            }
            GuiMessage::NativeFileDrop(drop) => self.apply_native_file_drop(drop, context),
            GuiMessage::Frame => {
                self.advance_frame();
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
        WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Started { .. }) => {
            Some("waveform.selection_export_drag.begin")
        }
        WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Ended { .. }) => {
            Some("waveform.selection_export_drag.end")
        }
        WaveformInteraction::FinishSelection { .. } => Some("waveform.selection.finish"),
        WaveformInteraction::UpdateSelection { .. }
        | WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Moved { .. })
        | WaveformInteraction::Frame => None,
    }
}

#[cfg(test)]
mod tests;
