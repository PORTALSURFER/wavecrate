//! Radiant-first Sempal application rebuilt incrementally beside the legacy sample.

use radiant::layout::Vector2;
use radiant::prelude as ui;
use radiant::runtime::{NativeRunOptions, NativeTextOptions};
use radiant::widgets::{DragHandleMessage, ScrollbarMessage, TextInputWidget, WidgetSizing};
use rfd::FileDialog;
use sempal::audio::AudioPlayer;
use sempal::gui_runtime::sempal_ui_font_path;
use std::{
    ffi::OsString,
    fs,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

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
enum RebuildMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    FolderScanProgress(FolderScanProgress),
    FolderScanDiscoveryBatch(FolderScanDiscoveryBatch),
    FolderScanFinished(FolderScanResult),
    SelectSample(String),
    PlaySelectedSample,
    FocusRenameInput(u64),
    NavigateBrowser(i32),
    CollapseSelectedFolder,
    ExpandSelectedFolder,
    Waveform(WaveformInteraction),
    Frame,
}

struct RebuildLayoutState {
    folder_width: f32,
    folder_resize: Option<FolderResize>,
    folder_browser: FolderBrowserState,
    waveform: WaveformState,
    sample_status: String,
    worker_sender: Sender<RebuildMessage>,
    worker_receiver: Option<Receiver<RebuildMessage>>,
    next_task_id: u64,
    folder_progress: Option<FolderScanProgress>,
    progress_tick: f32,
    audio_player: Option<AudioPlayer>,
}

impl RebuildLayoutState {
    fn load_default() -> Result<Self, String> {
        let (worker_sender, worker_receiver) = mpsc::channel();
        Ok(Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: FolderBrowserState::load_default(),
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Default sample loaded from assets"),
            worker_sender,
            worker_receiver: Some(worker_receiver),
            next_task_id: 1,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
        })
    }

    fn resize_folder_browser(&mut self, message: DragHandleMessage) {
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
    }

    fn apply_message(
        &mut self,
        message: RebuildMessage,
        context: &mut ui::UpdateContext<RebuildMessage>,
    ) {
        match message {
            RebuildMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            RebuildMessage::FolderBrowser(FolderBrowserMessage::AddSource) => {
                self.add_source_from_dialog(context);
            }
            RebuildMessage::FolderBrowser(FolderBrowserMessage::SelectSource(id)) => {
                self.select_source(id, context);
            }
            RebuildMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected) => {
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
                            RebuildMessage::FocusRenameInput(input_id),
                        );
                    }
                    Ok(None) => {
                        self.sample_status = String::from("Select a folder to rename");
                    }
                    Err(error) => {
                        self.sample_status = error;
                    }
                }
            }
            RebuildMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message)) => {
                if let Some(status) = self.folder_browser.apply_rename_input(message) {
                    self.sample_status = status;
                }
            }
            RebuildMessage::FolderBrowser(message) => self.folder_browser.apply_message(message),
            RebuildMessage::FolderScanProgress(progress) => {
                if self
                    .folder_browser
                    .scan_is_active(&progress.source_id, progress.task_id)
                {
                    self.folder_progress = Some(progress);
                }
            }
            RebuildMessage::FolderScanDiscoveryBatch(batch) => {
                self.folder_browser.apply_scan_discovered_batch(batch);
            }
            RebuildMessage::FolderScanFinished(result) => self.finish_folder_scan(result),
            RebuildMessage::SelectSample(path) => self.select_sample(path),
            RebuildMessage::PlaySelectedSample => self.play_selected_sample(),
            RebuildMessage::FocusRenameInput(input_id) => {
                context.focus(input_id);
            }
            RebuildMessage::NavigateBrowser(delta) => {
                if let Some(path) = self.folder_browser.navigate_vertical(delta) {
                    self.select_sample(path);
                }
            }
            RebuildMessage::CollapseSelectedFolder => {
                self.folder_browser.collapse_selected_folder();
            }
            RebuildMessage::ExpandSelectedFolder => {
                self.folder_browser.expand_selected_folder();
            }
            RebuildMessage::Waveform(message) => {
                self.waveform.apply_interaction(message);
                if let Some(start_ratio) = self.waveform.take_pending_playback_start() {
                    self.play_waveform_from_ratio(start_ratio);
                }
            }
            RebuildMessage::Frame => {
                self.waveform.apply_interaction(WaveformInteraction::Frame);
                self.refresh_playback_progress();
                if self.folder_progress.is_some() {
                    self.progress_tick = (self.progress_tick + 0.035) % 1.0;
                }
            }
        }
    }

    fn worker_subscription(&mut self) -> ui::Subscription<RebuildMessage> {
        self.worker_receiver
            .take()
            .map(|receiver| ui::Subscription::worker("rebuild-workers", receiver))
            .unwrap_or_else(ui::Subscription::none)
    }

    fn next_folder_task_id(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        task_id
    }

    fn add_source_from_dialog(&mut self, context: &mut ui::UpdateContext<RebuildMessage>) {
        let Some(path) = FileDialog::new().set_title("Add source").pick_folder() else {
            return;
        };
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_add_source_path(path, task_id) {
            self.launch_folder_scan(request, context);
        }
    }

    fn select_source(&mut self, id: String, context: &mut ui::UpdateContext<RebuildMessage>) {
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_select_source(id, task_id) {
            self.launch_folder_scan(request, context);
        }
    }

    fn launch_folder_scan(
        &mut self,
        request: FolderScanRequest,
        context: &mut ui::UpdateContext<RebuildMessage>,
    ) {
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
        let sender = self.worker_sender.clone();
        context.spawn(
            "rebuild-folder-scan",
            move || {
                let discovery_sender = sender.clone();
                let mut pending_discoveries = Vec::with_capacity(64);
                let task_id = request.task_id;
                let source_id = request.source_id.clone();
                let result = folder_browser::scan_source_with_progress(
                    request,
                    |progress| {
                        let _ = sender.send(RebuildMessage::FolderScanProgress(progress));
                    },
                    |event| {
                        pending_discoveries.push(event);
                        if pending_discoveries.len() >= 64 {
                            let events = std::mem::take(&mut pending_discoveries);
                            let _ =
                                discovery_sender.send(RebuildMessage::FolderScanDiscoveryBatch(
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
                    let _ = discovery_sender.send(RebuildMessage::FolderScanDiscoveryBatch(
                        FolderScanDiscoveryBatch {
                            task_id,
                            source_id,
                            events,
                        },
                    ));
                }
                result
            },
            RebuildMessage::FolderScanFinished,
        );
    }

    fn finish_folder_scan(&mut self, result: FolderScanResult) {
        let label = result.label.clone();
        let file_count = result.file_count;
        let folder_count = result.folder_count;
        if self.folder_browser.apply_scan_finished(result) {
            self.folder_progress = None;
            self.progress_tick = 0.0;
            self.sample_status =
                format!("Loaded source {label}: {file_count} files in {folder_count} folders");
        }
    }

    fn select_sample(&mut self, path: String) {
        match WaveformState::load_path(path.clone().into()) {
            Ok(waveform) => {
                self.folder_browser.select_file(path.clone());
                let file_name = waveform.file_name();
                self.waveform = waveform;
                match self.start_playback_path(Path::new(&path), 0.0) {
                    Ok(()) => {
                        self.sample_status = format!("Playing {file_name}");
                    }
                    Err(err) => {
                        self.sample_status =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                    }
                }
            }
            Err(err) => {
                self.sample_status = format!("Could not load sample: {err}");
            }
        }
    }

    fn play_selected_sample(&mut self) {
        let path = self
            .folder_browser
            .selected_file_id()
            .map(Path::new)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.waveform.path());
        match self.start_playback_path(&path, 0.0) {
            Ok(()) => {
                self.sample_status = format!("Playing {}", self.waveform.file_name());
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
            }
        }
    }

    fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
        let path = self.waveform.path();
        match self.start_playback_path(&path, start_ratio) {
            Ok(()) => {
                self.sample_status = format!(
                    "Playing {} from {:.1}%",
                    self.waveform.file_name(),
                    start_ratio * 100.0
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
            }
        }
    }

    fn start_playback_path(&mut self, path: &Path, start_ratio: f32) -> Result<(), String> {
        let bytes = fs::read(path).map_err(|err| format!("failed to read sample: {err}"))?;
        if self.audio_player.is_none() {
            self.audio_player = Some(AudioPlayer::new()?);
        }
        let start_ratio = start_ratio.clamp(0.0, 1.0);
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        player.set_audio(bytes, duration);
        player.play_from_fraction(f64::from(start_ratio))?;
        self.waveform.start_playback(start_ratio);
        Ok(())
    }

    fn refresh_playback_progress(&mut self) {
        let Some(player) = self.audio_player.as_mut() else {
            return;
        };
        if let Some(error) = player.take_error() {
            self.waveform.stop_playback();
            self.sample_status = format!("Playback stopped: {error}");
            return;
        }
        if player.is_playing() {
            if let Some(progress) = player.progress() {
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
            self.waveform.stop_playback();
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FolderResize {
    start_x: f32,
    start_width: f32,
}

/// Run the new Radiant-first application shell.
pub(crate) fn run() -> Result<(), String> {
    let options = NativeRunOptions {
        title: String::from("Sempal"),
        inner_size: Some([960.0, 540.0]),
        min_inner_size: Some([640.0, 360.0]),
        debug_layout: debug_layout_requested(std::env::args_os()),
        text: NativeTextOptions {
            font_paths: vec![sempal_ui_font_path()],
        },
        ..NativeRunOptions::default()
    };

    radiant::app(RebuildLayoutState::load_default()?)
        .options(options)
        .view(view)
        .animation(|state| state.waveform.is_playing() || state.folder_progress.is_some())
        .on_frame(|| RebuildMessage::Frame)
        .subscriptions(RebuildLayoutState::worker_subscription)
        .shortcuts(|state, _, press, _| {
            if state.folder_browser.rename_active() {
                ui::ShortcutResolution::unhandled()
            } else if press == ui::KeyPress::new(ui::KeyCode::F2) {
                ui::ShortcutResolution::action(RebuildMessage::FolderBrowser(
                    FolderBrowserMessage::BeginRenameSelected,
                ))
            } else if press == ui::KeyPress::new(ui::KeyCode::Space) {
                ui::ShortcutResolution::action(RebuildMessage::PlaySelectedSample)
            } else if press == ui::KeyPress::new(ui::KeyCode::ArrowUp) {
                ui::ShortcutResolution::action(RebuildMessage::NavigateBrowser(-1))
            } else if press == ui::KeyPress::new(ui::KeyCode::ArrowDown) {
                ui::ShortcutResolution::action(RebuildMessage::NavigateBrowser(1))
            } else if press == ui::KeyPress::new(ui::KeyCode::ArrowLeft) {
                ui::ShortcutResolution::action(RebuildMessage::CollapseSelectedFolder)
            } else if press == ui::KeyPress::new(ui::KeyCode::ArrowRight) {
                ui::ShortcutResolution::action(RebuildMessage::ExpandSelectedFolder)
            } else {
                ui::ShortcutResolution::unhandled()
            }
        })
        .update_with(|state, message, context| {
            state.apply_message(message, context);
            context.request_repaint();
        })
        .run()
}

fn debug_layout_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .any(|arg| arg == DEBUG_LAYOUT_ARG || arg == DEBUG_LAYOUT_SHORT_ARG)
}

fn view(state: &mut RebuildLayoutState) -> ui::View<RebuildMessage> {
    ui::column([
        top_status_bar(),
        center_panel(state),
        bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill()
}

fn top_status_bar() -> ui::View<RebuildMessage> {
    ui::row([
        ui::text("Sempal").height(20.0).width(120.0),
        ui::text("Radiant rebuild").height(20.0).fill_width(),
        ui::text("ready").height(20.0).width(80.0),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn center_panel(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
    ui::row([folder_sidebar(state), folder_splitter(), main_area(state)])
        .padding(6.0)
        .fill()
}

fn folder_sidebar(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
    folder_browser::folder_browser_view(&state.folder_browser)
        .width(state.folder_width)
        .fill_height()
}

fn folder_splitter() -> ui::View<RebuildMessage> {
    ui::column([
        ui::spacer().fill(),
        ui::drag_handle()
            .mapped(RebuildMessage::ResizeFolder)
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

fn main_area(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
    ui::column([
        main_toolbar(state),
        waveform_panel(state),
        sample_browser(state),
    ])
    .padding(4.0)
    .fill()
}

fn main_toolbar(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
    let audio_count = state.folder_browser.selected_audio_files().len();
    ui::row([
        ui::text("Source").height(22.0).width(80.0),
        ui::text("assets/portal_SS_kick_003.wav")
            .height(22.0)
            .fill_width()
            .truncate(),
        ui::text(format!("{audio_count} audio"))
            .height(22.0)
            .width(96.0),
    ])
    .padding_y(3.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(34.0)
}

fn waveform_panel(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
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
    format!(
        "{} | {} Hz | {} channel{} -> mono | {} frames",
        waveform.file_name(),
        waveform.sample_rate(),
        waveform.channels(),
        if waveform.channels() == 1 { "" } else { "s" },
        waveform.frames()
    )
}

fn waveform_scrollbar(waveform: &WaveformState) -> ui::View<RebuildMessage> {
    let mut scrollbar = radiant::widgets::ScrollbarWidget::new(
        0,
        radiant::widgets::ScrollbarAxis::Horizontal,
        radiant::widgets::WidgetSizing::fixed(radiant::gui::types::Vector2::new(1200.0, 12.0)),
    );
    scrollbar.props.viewport_fraction = waveform.visible_fraction();
    scrollbar.state.offset_fraction = waveform.offset_fraction();
    ui::custom_widget(scrollbar, |output| {
        output
            .typed_ref::<ScrollbarMessage>()
            .copied()
            .map(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => {
                    RebuildMessage::Waveform(WaveformInteraction::ScrollTo { offset_fraction })
                }
            })
    })
    .fill_width()
    .height(12.0)
}

fn sample_browser(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
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

fn sample_browser_header() -> ui::View<RebuildMessage> {
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

fn sample_header_cell(label: &str, width: f32) -> ui::View<RebuildMessage> {
    ui::text(label).height(20.0).width(width)
}

fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
) -> ui::View<RebuildMessage> {
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
) -> ui::View<RebuildMessage> {
    let row = compact_details_row([
        sample_name_cell(file, rename),
        sample_file_cell(file, file.extension.clone(), SAMPLE_EXT_WIDTH, "extension"),
        sample_file_cell(file, file.size.clone(), SAMPLE_SIZE_WIDTH, "size"),
        sample_file_cell(
            file,
            file.modified.clone(),
            SAMPLE_MODIFIED_WIDTH,
            "modified",
        ),
    ])
    .key(format!("sample-row-{}", file.id))
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
) -> ui::View<RebuildMessage> {
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
        RebuildMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
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
) -> ui::View<RebuildMessage> {
    ui::button(value)
        .message(RebuildMessage::SelectSample(file.id.clone()))
        .key(format!("sample-{}-{column_id}", file.id))
        .fill_width()
        .height(20.0)
        .input_only()
        .width(width)
}

fn compact_details_row(
    children: impl IntoIterator<Item = ui::View<RebuildMessage>>,
) -> ui::View<RebuildMessage> {
    ui::row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

fn details_header_row(
    children: impl IntoIterator<Item = ui::View<RebuildMessage>>,
) -> ui::View<RebuildMessage> {
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

fn sample_browser_status(audio_count: usize) -> ui::View<RebuildMessage> {
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

fn bottom_status_bar(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
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

fn bottom_status_text(state: &RebuildLayoutState) -> String {
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

fn worker_progress_bar(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
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

#[cfg(test)]
mod tests {
    use super::{
        DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH,
        MIN_FOLDER_WIDTH, RebuildLayoutState, debug_layout_requested,
    };
    use radiant::{gui::types::Point, prelude as ui, widgets::DragHandleMessage};
    use std::{ffi::OsString, sync::mpsc};

    #[test]
    fn canonical_debug_layout_arg_enables_new_ui_overlay() {
        assert!(debug_layout_requested([
            OsString::from("sempal"),
            OsString::from(DEBUG_LAYOUT_ARG),
        ]));
    }

    #[test]
    fn short_debug_layout_arg_enables_new_ui_overlay() {
        assert!(debug_layout_requested([
            OsString::from("sempal"),
            OsString::from(DEBUG_LAYOUT_SHORT_ARG),
        ]));
    }

    #[test]
    fn unrelated_args_leave_new_ui_overlay_disabled() {
        assert!(!debug_layout_requested([
            OsString::from("sempal"),
            OsString::from("--debug-log"),
        ]));
    }

    #[test]
    fn folder_browser_splitter_resizes_and_clamps_width() {
        let mut state = RebuildLayoutState {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: super::FolderBrowserState::load_default(),
            waveform: super::WaveformState::synthetic_for_tests(),
            sample_status: String::new(),
            worker_sender: mpsc::channel().0,
            worker_receiver: None,
            next_task_id: 1,
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
    fn default_waveform_sample_is_bundled_asset() {
        let path = super::waveform::default_sample_path();
        assert!(path.ends_with("assets/portal_SS_kick_003.wav"));
        assert!(path.is_file());
    }

    #[test]
    fn default_waveform_sample_loads_for_rebuild_ui() {
        let waveform = super::WaveformState::load_default().expect("default sample loads");
        assert!(waveform.frames() > 0);
        assert!(waveform.sample_rate() > 0);
    }

    #[test]
    fn sample_selection_loads_selected_file_into_waveform() {
        let mut state = RebuildLayoutState {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: super::FolderBrowserState::load_default(),
            waveform: super::WaveformState::synthetic_for_tests(),
            sample_status: String::new(),
            worker_sender: mpsc::channel().0,
            worker_receiver: None,
            next_task_id: 1,
            folder_progress: None,
            progress_tick: 0.0,
            audio_player: None,
        };
        let sample_path = state.folder_browser.selected_audio_files()[0].id.clone();

        let mut context = ui::UpdateContext::default();
        state.apply_message(
            super::RebuildMessage::SelectSample(sample_path.clone()),
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
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["portal_SS_kick_003.wav"]
        );
    }
}
