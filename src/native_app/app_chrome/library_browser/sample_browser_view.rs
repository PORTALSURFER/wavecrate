use radiant::prelude as ui;

#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app::{GuiMessage, SampleNameViewMode};
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel;
use crate::native_app::sample_library::folder_browser::{
    FileColumn, FileColumnDragFeedback, FolderBrowserMessage,
};
use crate::native_app::ui::ids as widget_ids;

mod hit_target;
pub(in crate::native_app) use hit_target::SampleFileHitTarget;

mod row_projection;
mod row_widgets;
mod rows;
use rows::sample_browser_rows;

const SAMPLE_HEADER_SORT_DRAG_SCOPE: u64 = widget_ids::SAMPLE_HEADER_SORT_DRAG_ID;
const SAMPLE_HEADER_RESIZE_SCOPE: u64 = widget_ids::SAMPLE_HEADER_RESIZE_ID;

#[cfg(test)]
pub(in crate::native_app) fn sample_browser_from_state(
    state: &mut NativeAppState,
) -> ui::View<GuiMessage> {
    sample_browser(SampleBrowserViewModel::from_app_state(state))
}

pub(in crate::native_app) fn sample_browser(
    model: SampleBrowserViewModel<'_>,
) -> ui::View<GuiMessage> {
    ui::column([
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
        ),
        sample_browser_status(model.audio_count),
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
