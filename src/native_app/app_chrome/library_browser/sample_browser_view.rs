use radiant::prelude as ui;

#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app::{GuiMessage, SampleNameViewMode};
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel;
use crate::native_app::sample_library::folder_browser::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::FileColumn;
use crate::native_app::sample_library::folder_browser::projection::FileColumnDragFeedback;
use crate::native_app::ui::ids as widget_ids;

mod hit_target;
pub(in crate::native_app) use hit_target::SampleFileHitTarget;

mod row_projection;
mod row_widgets;
mod rows;
use rows::sample_browser_rows;

const SAMPLE_HEADER_SORT_DRAG_SCOPE: u64 = widget_ids::SAMPLE_HEADER_SORT_DRAG_ID;
const SAMPLE_HEADER_RESIZE_SCOPE: u64 = widget_ids::SAMPLE_HEADER_RESIZE_ID;
const SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH: f32 = 22.0;
pub(super) const SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH: f32 = 58.0;

#[cfg(test)]
pub(in crate::native_app) fn sample_browser_from_state(
    state: &mut NativeAppState,
) -> ui::View<GuiMessage> {
    SampleBrowserViewModel::prepare_visible_sample_window(state);
    sample_browser(SampleBrowserViewModel::from_app_state(state))
}

pub(in crate::native_app) fn sample_browser(
    model: SampleBrowserViewModel<'_>,
) -> ui::View<GuiMessage> {
    ui::column([
        sample_browser_header_bar(
            &model.visible_samples.columns,
            model.visible_samples.sort,
            model.drag_feedback.as_ref(),
            model.name_view_mode,
            model.visible_samples.similarity_mode_active,
        ),
        sample_browser_rows(
            &model.visible_samples,
            model.name_view_mode,
            model.metadata_tags_by_file,
        ),
        sample_browser_status(model.visible_samples.total_count),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill()
    .pointer_target_opt(sample_list_browser_drag_cancel_target(
        model.file_drag_active,
    ))
    .pointer_target_opt(sample_list_waveform_drop_target(
        model.extracted_file_drag_active,
    ))
    .pointer_target_opt(sample_list_clear_folder_drop_target(
        model.hovered_folder_drop_target,
    ))
}

fn sample_list_browser_drag_cancel_target(active: bool) -> Option<ui::PointerTarget<GuiMessage>> {
    active.then(|| {
        ui::pointer_target(true)
            .pointer_move(false)
            .pointer_press(false)
            .wheel(false)
            .filter_map(|message| match message {
                ui::PointerShieldMessage::PointerRelease { .. }
                | ui::PointerShieldMessage::PointerDrop { .. } => {
                    Some(GuiMessage::CancelBrowserDragOnSampleList)
                }
                ui::PointerShieldMessage::PointerMove { .. }
                | ui::PointerShieldMessage::PointerPress { .. } => {
                    Some(GuiMessage::CancelBrowserDragOnSampleList)
                }
                ui::PointerShieldMessage::Wheel { .. } => None,
            })
            .key("sample-list-browser-drag-cancel-target")
    })
}

fn sample_list_waveform_drop_target(active: bool) -> Option<ui::PointerTarget<GuiMessage>> {
    active.then(|| {
        ui::pointer_drop_target(true)
            .on_drop(GuiMessage::DropWaveformSelectionOnSampleList)
            .key("sample-list-waveform-drop-target")
    })
}

fn sample_list_clear_folder_drop_target(active: bool) -> Option<ui::PointerTarget<GuiMessage>> {
    active.then(|| {
        ui::pointer_move_target(true)
            .on_pointer_move(|position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position))
            })
            .key("sample-list-clear-folder-drop-target")
    })
}

fn sample_browser_header_bar(
    columns: &[&FileColumn],
    sort: &ui::DetailsSort,
    drag_feedback: Option<&FileColumnDragFeedback>,
    mode: SampleNameViewMode,
    similarity_mode_active: bool,
) -> ui::View<GuiMessage> {
    ui::row([
        sample_browser_header(columns, sort, drag_feedback, similarity_mode_active).fill_width(),
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
    similarity_mode_active: bool,
) -> ui::View<GuiMessage> {
    let header_cells = columns
        .iter()
        .flat_map(|column| sample_header_cells(column, sort, similarity_mode_active));
    let header = ui::row([
        ui::spacer()
            .width(SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH)
            .height(24.0),
        ui::compact_details_header_row(header_cells).fill_width(),
    ])
    .spacing(0.0)
    .fill_width()
    .height(24.0);
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

fn sample_header_cells(
    column: &FileColumn,
    sort: &ui::DetailsSort,
    similarity_mode_active: bool,
) -> Vec<ui::View<GuiMessage>> {
    let mut cells = vec![sample_header_cell(column, sort)];
    if column.id == "name" && similarity_mode_active {
        cells.push(sample_similarity_header_cell());
    }
    cells
}

fn sample_similarity_header_cell() -> ui::View<GuiMessage> {
    ui::compact_details_cell(
        ui::text("Sim")
            .muted_text()
            .key("sample-header-similarity-label")
            .height(20.0),
        Some(SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH),
    )
    .key("sample-header-similarity")
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
