//! Radiant-first Sempal application rebuilt incrementally beside the legacy sample.

use radiant::prelude as ui;
use radiant::runtime::{NativeRunOptions, NativeTextOptions};
use radiant::widgets::{DragHandleMessage, ScrollbarMessage};
use sempal::gui_runtime::sempal_ui_font_path;
use std::ffi::OsString;

mod folder_browser;
mod waveform;
use folder_browser::{FileEntry, FolderBrowserMessage, FolderBrowserState};
use waveform::{WaveformInteraction, WaveformState};

const DEBUG_LAYOUT_ARG: &str = "--debug-layout";
const DEBUG_LAYOUT_SHORT_ARG: &str = "-debug-layout";
const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;

#[derive(Clone, Debug, PartialEq)]
enum RebuildMessage {
    ResizeFolder(DragHandleMessage),
    FolderBrowser(FolderBrowserMessage),
    SelectSample(String),
    Waveform(WaveformInteraction),
    Frame,
}

#[derive(Clone, Debug)]
struct RebuildLayoutState {
    folder_width: f32,
    folder_resize: Option<FolderResize>,
    folder_browser: FolderBrowserState,
    waveform: WaveformState,
    sample_status: String,
}

impl RebuildLayoutState {
    fn load_default() -> Result<Self, String> {
        Ok(Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: FolderBrowserState::load_default(),
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Default sample loaded from assets"),
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

    fn apply_message(&mut self, message: RebuildMessage) {
        match message {
            RebuildMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            RebuildMessage::FolderBrowser(message) => self.folder_browser.apply_message(message),
            RebuildMessage::SelectSample(path) => self.select_sample(path),
            RebuildMessage::Waveform(message) => self.waveform.apply_interaction(message),
            RebuildMessage::Frame => self.waveform.apply_interaction(WaveformInteraction::Frame),
        }
    }

    fn select_sample(&mut self, path: String) {
        match WaveformState::load_path(path.clone().into()) {
            Ok(waveform) => {
                self.folder_browser.select_file(path);
                let file_name = waveform.file_name();
                self.waveform = waveform;
                self.sample_status = format!("Loaded {file_name}");
            }
            Err(err) => {
                self.sample_status = format!("Could not load sample: {err}");
            }
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
        .animation(|state| state.waveform.is_playing())
        .on_frame(|| RebuildMessage::Frame)
        .update_with(|state, message, context| {
            state.apply_message(message);
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
            .height(86.0),
        waveform_scrollbar(&state.waveform),
        waveform_controls(),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(150.0)
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

fn waveform_controls() -> ui::View<RebuildMessage> {
    ui::row([
        ui::button("Zoom -")
            .subtle()
            .message(RebuildMessage::Waveform(WaveformInteraction::Zoom {
                factor: 2.0,
            })),
        ui::button("Zoom +")
            .primary()
            .message(RebuildMessage::Waveform(WaveformInteraction::Zoom {
                factor: 0.5,
            })),
        ui::button("Pan <")
            .subtle()
            .message(RebuildMessage::Waveform(WaveformInteraction::Pan {
                visible_fraction: -0.25,
            })),
        ui::button("Pan >")
            .subtle()
            .message(RebuildMessage::Waveform(WaveformInteraction::Pan {
                visible_fraction: 0.25,
            })),
        ui::button("Play")
            .subtle()
            .message(RebuildMessage::Waveform(
                WaveformInteraction::TogglePlayback,
            )),
        ui::button("Reset")
            .subtle()
            .message(RebuildMessage::Waveform(WaveformInteraction::Reset)),
        ui::spacer().fill(),
    ])
    .spacing(6.0)
    .fill_width()
    .height(28.0)
}

fn sample_browser(state: &RebuildLayoutState) -> ui::View<RebuildMessage> {
    let audio_files = state.folder_browser.selected_audio_files();
    ui::column([
        sample_browser_header(),
        sample_browser_rows(&audio_files, state.folder_browser.selected_file_id()),
        sample_browser_status(audio_files.len()),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill()
}

fn sample_browser_header() -> ui::View<RebuildMessage> {
    details_header_row([
        sample_header_cell("Name", SAMPLE_NAME_WIDTH),
        sample_header_cell("Size", SAMPLE_SIZE_WIDTH),
        sample_header_cell("Type", SAMPLE_TYPE_WIDTH),
        sample_header_cell("Modified", SAMPLE_MODIFIED_WIDTH),
    ])
}

const SAMPLE_NAME_WIDTH: f32 = 240.0;
const SAMPLE_SIZE_WIDTH: f32 = 78.0;
const SAMPLE_TYPE_WIDTH: f32 = 118.0;
const SAMPLE_MODIFIED_WIDTH: f32 = 112.0;

fn sample_header_cell(label: &str, width: f32) -> ui::View<RebuildMessage> {
    ui::text(label).height(20.0).width(width)
}

fn sample_browser_rows(
    files: &[&FileEntry],
    selected_file_id: Option<&str>,
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
                .map(|file| sample_browser_row(file, selected_file_id == Some(file.id.as_str())))
                .collect::<Vec<_>>(),
        )
        .spacing(1.0)
        .fill_width(),
    )
    .fill()
}

fn sample_browser_row(file: &FileEntry, selected: bool) -> ui::View<RebuildMessage> {
    let row = compact_details_row([
        sample_file_cell(file, file.name.clone(), SAMPLE_NAME_WIDTH, "name"),
        sample_file_cell(file, file.size.clone(), SAMPLE_SIZE_WIDTH, "size"),
        sample_file_cell(file, file.kind.clone(), SAMPLE_TYPE_WIDTH, "kind"),
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
        ui::text(state.sample_status.clone())
            .height(20.0)
            .fill_width(),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

#[cfg(test)]
mod tests {
    use super::{
        debug_layout_requested, RebuildLayoutState, DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG,
        DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
    };
    use radiant::{gui::types::Point, widgets::DragHandleMessage};
    use std::ffi::OsString;

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
        };
        let sample_path = state.folder_browser.selected_audio_files()[0].id.clone();

        state.apply_message(super::RebuildMessage::SelectSample(sample_path.clone()));

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
        assert!(browser
            .selected_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_003.wav"));
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
