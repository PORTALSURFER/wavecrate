use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, PointerModifiers, Widget,
    WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

use super::folder_browser::{
    self, FileColumn, FileEntry, FolderBrowserMessage, FolderBrowserState,
};
use super::{
    GuiAppState, GuiMessage, SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID,
    SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
    SAMPLE_BROWSER_ROW_HEIGHT,
};

pub(super) fn sample_browser(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let window = state.folder_browser.follow_selected_file_view(
        SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
        SAMPLE_BROWSER_OVERSCAN_ROWS,
        SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
    );
    let audio_files = state.folder_browser.selected_audio_files();
    let audio_count = audio_files.len();
    let columns = state.folder_browser.visible_file_columns();
    ui::column([
        sample_browser_header(&columns, state.folder_browser.file_sort()),
        sample_browser_rows(&state.folder_browser, &audio_files, &columns, window),
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
    window: ui::VirtualListWindow,
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return ui::text("No audio files in selected folder")
            .height(24.0)
            .fill_width()
            .fill_height();
    }

    ui::virtual_list_window(
        window,
        SAMPLE_BROWSER_ROW_HEIGHT,
        |index| {
            let file = files[index];
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
                SampleFileHitMessage::ContextMenu(position) => GuiMessage::OpenSampleContextMenu {
                    path: hit_path.clone(),
                    position,
                },
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
    ui::text_input(rename.draft)
        .selection(rename.selection_start, rename.selection_end)
        .message_event(|message| {
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
pub(super) struct SampleFileHitTarget {
    common: WidgetCommon,
    selected: bool,
    dragged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SampleFileHitMessage {
    Activate(PointerModifiers),
    ContextMenu(Point),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    pub(super) fn new(selected: bool) -> Self {
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
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = false;
                self.dragged = false;
                Some(WidgetOutput::typed(SampleFileHitMessage::ContextMenu(
                    position,
                )))
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
