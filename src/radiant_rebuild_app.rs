//! Radiant-first Sempal application rebuilt incrementally beside the legacy sample.

use radiant::prelude as ui;
use radiant::runtime::{NativeRunOptions, NativeTextOptions};
use radiant::widgets::DragHandleMessage;
use sempal::gui_runtime::sempal_ui_font_path;
use std::ffi::OsString;

const DEBUG_LAYOUT_ARG: &str = "--debug-layout";
const DEBUG_LAYOUT_SHORT_ARG: &str = "-debug-layout";
const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;

#[derive(Clone, Copy, Debug)]
struct RebuildLayoutState {
    folder_width: f32,
    folder_resize: Option<FolderResize>,
}

impl Default for RebuildLayoutState {
    fn default() -> Self {
        Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FolderResize {
    start_x: f32,
    start_width: f32,
}

impl RebuildLayoutState {
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

    radiant::app(RebuildLayoutState::default())
        .options(options)
        .view(view)
        .run()
}

fn debug_layout_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .any(|arg| arg == DEBUG_LAYOUT_ARG || arg == DEBUG_LAYOUT_SHORT_ARG)
}

fn view(state: &mut RebuildLayoutState) -> ui::StateView<RebuildLayoutState> {
    ui::column([top_status_bar(), center_panel(state), bottom_status_bar()])
        .spacing(0.0)
        .fill()
}

fn top_status_bar() -> ui::StateView<RebuildLayoutState> {
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

fn center_panel(state: &RebuildLayoutState) -> ui::StateView<RebuildLayoutState> {
    ui::row([folder_sidebar(state), folder_splitter(), main_area()])
        .padding(6.0)
        .fill()
}

fn folder_sidebar(state: &RebuildLayoutState) -> ui::StateView<RebuildLayoutState> {
    ui::column([
        ui::text("Folders").height(22.0).fill_width(),
        ui::spacer().fill(),
    ])
    .spacing(3.0)
    .padding(4.0)
    .width(state.folder_width)
    .fill_height()
}

fn folder_splitter() -> ui::StateView<RebuildLayoutState> {
    ui::column([
        ui::spacer().fill(),
        ui::drag_handle()
            .on_drag(|state: &mut RebuildLayoutState, message| {
                state.resize_folder_browser(message);
            })
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

fn main_area() -> ui::StateView<RebuildLayoutState> {
    ui::column([main_toolbar(), waveform_panel(), sample_browser()])
        .padding(4.0)
        .fill()
}

fn main_toolbar() -> ui::StateView<RebuildLayoutState> {
    ui::row([
        ui::text("Source").height(22.0).width(80.0),
        ui::text("No folder loaded").height(22.0).fill_width(),
        ui::text("0 selected").height(22.0).width(96.0),
    ])
    .padding_y(3.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(34.0)
}

fn waveform_panel() -> ui::StateView<RebuildLayoutState> {
    ui::column([
        ui::text("Waveform").height(20.0).fill_width(),
        ui::spacer().fill(),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(150.0)
}

fn sample_browser() -> ui::StateView<RebuildLayoutState> {
    ui::column([
        sample_browser_header(),
        ui::spacer().fill(),
        sample_browser_status(),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill()
}

fn sample_browser_header() -> ui::StateView<RebuildLayoutState> {
    ui::row([
        ui::text("Name").height(22.0).fill_width(),
        ui::text("Type").height(22.0).width(120.0),
        ui::text("Length").height(22.0).width(90.0),
        ui::text("Tags").height(22.0).width(140.0),
    ])
    .padding_x(3.0)
    .fill_width()
    .height(28.0)
}

fn sample_browser_status() -> ui::StateView<RebuildLayoutState> {
    ui::row([
        ui::text("Browser").height(20.0).width(90.0),
        ui::text("Samples will be listed here")
            .height(20.0)
            .fill_width(),
    ])
    .padding_x(3.0)
    .fill_width()
    .height(28.0)
}

fn bottom_status_bar() -> ui::StateView<RebuildLayoutState> {
    ui::row([
        ui::text("0 samples").height(20.0).width(120.0),
        ui::text("No source loaded").height(20.0).fill_width(),
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
        let mut state = RebuildLayoutState::default();
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
}
