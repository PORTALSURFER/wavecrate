use radiant::prelude as ui;
use std::collections::{HashMap, HashSet};

use crate::native_app::app::{GuiMessage, NativeAppState, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::{
    FileColumn, FileColumnDragFeedback, FolderBrowserMessage, FolderBrowserState,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::ui::ids as widget_ids;

mod hit_target;
pub(in crate::native_app) use hit_target::SampleFileHitTarget;

mod row_widgets;
mod rows;
use rows::sample_browser_rows;

const SAMPLE_HEADER_SORT_DRAG_SCOPE: u64 = widget_ids::SAMPLE_HEADER_SORT_DRAG_ID;
const SAMPLE_HEADER_RESIZE_SCOPE: u64 = widget_ids::SAMPLE_HEADER_RESIZE_ID;

pub(in crate::native_app) struct SampleBrowserViewModel<'a> {
    folder_browser: &'a FolderBrowserState,
    audio_count: usize,
    columns: Vec<&'a FileColumn>,
    window: ui::VirtualListWindow,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &'a HashMap<String, Vec<String>>,
    cached_sample_paths: &'a HashSet<String>,
    suppress_row_hover: bool,
    file_drag_active: bool,
    extracted_file_drag_active: bool,
    hovered_folder_drop_target: bool,
    drag_feedback: Option<FileColumnDragFeedback>,
}

impl<'a> SampleBrowserViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(
        state: &'a mut NativeAppState,
        suppress_row_hover: bool,
    ) -> Self {
        let window = state
            .folder_browser
            .follow_selected_file_view_matching_tags(
                SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
                SAMPLE_BROWSER_OVERSCAN_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                &state.metadata_tags_by_file,
            );
        let audio_count = state
            .folder_browser
            .selected_audio_file_count_matching_tags(&state.metadata_tags_by_file);
        let columns = state.folder_browser.visible_file_columns();
        let file_drag_active = state.folder_browser.file_drag_active();
        let extracted_file_drag_active = state.folder_browser.extracted_file_drag_active();
        let hovered_folder_drop_target = state
            .folder_browser
            .hovered_drop_target_folder_id()
            .is_some();
        let drag_feedback = state.folder_browser.file_column_drag_feedback();

        Self {
            folder_browser: &state.folder_browser,
            audio_count,
            columns,
            window,
            name_view_mode: state.sample_name_view_mode,
            metadata_tags_by_file: &state.metadata_tags_by_file,
            cached_sample_paths: &state.cached_sample_paths,
            suppress_row_hover,
            file_drag_active,
            extracted_file_drag_active,
            hovered_folder_drop_target,
            drag_feedback,
        }
    }
}

#[cfg(test)]
pub(in crate::native_app) fn sample_browser_from_state(
    state: &mut NativeAppState,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    sample_browser(SampleBrowserViewModel::from_app_state(
        state,
        suppress_row_hover,
    ))
}

pub(in crate::native_app) fn sample_browser(
    model: SampleBrowserViewModel<'_>,
) -> ui::View<GuiMessage> {
    let browser = ui::column([
        sample_browser_header_bar(
            &model.columns,
            model.folder_browser.file_sort(),
            model.drag_feedback.as_ref(),
            model.name_view_mode,
        ),
        sample_browser_rows(
            model.folder_browser,
            model.audio_count,
            &model.columns,
            model.window,
            model.name_view_mode,
            model.metadata_tags_by_file,
            model.cached_sample_paths,
            model.suppress_row_hover,
        ),
        sample_browser_status(model.audio_count),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill();
    if !model.file_drag_active
        && !model.extracted_file_drag_active
        && !model.hovered_folder_drop_target
    {
        return browser;
    }
    let mut layers = vec![browser];
    if model.file_drag_active {
        layers.push(
            ui::pointer_shield(true)
                .pointer_move(false)
                .pointer_press(false)
                .mapped(|message| match message {
                    ui::PointerShieldMessage::PointerRelease { .. }
                    | ui::PointerShieldMessage::PointerDrop { .. } => {
                        GuiMessage::CancelBrowserDragOnSampleList
                    }
                    ui::PointerShieldMessage::PointerMove { .. }
                    | ui::PointerShieldMessage::PointerPress { .. } => {
                        GuiMessage::CancelBrowserDragOnSampleList
                    }
                })
                .key("sample-list-browser-drag-cancel-target")
                .input_only()
                .fill(),
        );
    }
    if model.extracted_file_drag_active {
        layers.push(
            ui::pointer_drop_shield(true)
                .on_drop(GuiMessage::DropWaveformSelectionOnSampleList)
                .key("sample-list-waveform-drop-target")
                .input_only()
                .fill(),
        );
    }
    if model.hovered_folder_drop_target {
        layers.push(
            ui::pointer_move_shield(true)
                .on_pointer_move(|position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position))
                })
                .key("sample-list-clear-folder-drop-target")
                .input_only()
                .fill(),
        );
    }
    ui::stack(layers).fill()
}

fn sample_browser_header_bar(
    columns: &[&FileColumn],
    sort: &ui::DetailsSort,
    drag_feedback: Option<&FileColumnDragFeedback>,
    mode: SampleNameViewMode,
) -> ui::View<GuiMessage> {
    ui::row([
        sample_browser_header(columns, sort, drag_feedback).fill_width(),
        sample_name_view_mode_button(mode),
    ])
    .fill_width()
    .height(24.0)
    .spacing(6.0)
}

fn sample_name_view_mode_button(mode: SampleNameViewMode) -> ui::View<GuiMessage> {
    let label = match mode {
        SampleNameViewMode::DiskFilename => "Disk",
        SampleNameViewMode::MetadataLabel => "Label",
    };
    ui::button(label)
        .message(GuiMessage::ToggleSampleNameViewMode)
        .key("sample-name-view-mode-toggle")
        .size(58.0, 22.0)
}

fn sample_browser_header(
    columns: &[&FileColumn],
    sort: &ui::DetailsSort,
    drag_feedback: Option<&FileColumnDragFeedback>,
) -> ui::View<GuiMessage> {
    let header = ui::compact_details_header_row(
        columns
            .iter()
            .map(|column| sample_header_cell(column, sort)),
    );
    let Some(feedback) = drag_feedback else {
        return header;
    };
    ui::stack([header, column_drop_marker(feedback.marker_x)])
        .fill_width()
        .height(24.0)
}

fn column_drop_marker(x: f32) -> ui::View<GuiMessage> {
    ui::local_drop_marker(x, ui::Rgba8::new(255, 160, 82, 230), 2.0, 20.0)
        .key("sample-column-drop-marker")
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
}

fn sample_header_cell(column: &FileColumn, sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    let sort_id = column.id.clone();
    let drag_id = column.id.clone();
    let resize_id = column.id.clone();
    let label = ui::details_sort_label(column.label.as_str(), column.id.as_str(), Some(sort));
    ui::compact_resizable_details_header_cell_with_ids(
        format!("sample-header-{}", column.id),
        label,
        column.width,
        ui::CompactDetailsHeaderCellIds::new(
            Some(ui::stable_widget_id(
                SAMPLE_HEADER_SORT_DRAG_SCOPE,
                column.id.as_str(),
            )),
            Some(ui::stable_widget_id(
                SAMPLE_HEADER_RESIZE_SCOPE,
                column.id.as_str(),
            )),
        ),
        GuiMessage::FolderBrowser(FolderBrowserMessage::SortFileColumn(sort_id)),
        move |drag| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::DragFileColumn(drag_id.clone(), drag))
        },
        move |message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFileColumn(
                resize_id.clone(),
                message,
            ))
        },
    )
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
