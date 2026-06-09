use radiant::prelude as ui;
use std::collections::HashMap;
use std::collections::HashSet;

use super::SampleFileHitTarget;
use super::row_projection::{
    SampleColumnContent, SampleColumnDisplay, SampleRowDisplay, sample_row_display,
};
use super::row_widgets::RatingIndicator;
use crate::native_app::app::{GuiMessage, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::{
    FileColumn, FileRenameView, FolderBrowserMessage, FolderBrowserState,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
};

pub(super) fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    file_count: usize,
    columns: &[&FileColumn],
    window: ui::VirtualListWindow,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached_sample_paths: &HashSet<String>,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    if file_count == 0 {
        return empty_sample_browser_rows();
    }

    ui::virtual_list_windowed(|index| {
        let Some(file) =
            folder_browser.selected_audio_file_at_matching_tags(index, metadata_tags_by_file)
        else {
            return ui::empty().fill_width().height(SAMPLE_BROWSER_ROW_HEIGHT);
        };
        sample_browser_row(sample_row_display(
            file,
            folder_browser,
            columns,
            name_view_mode,
            metadata_tags_by_file,
            cached_sample_paths.contains(&file.id),
            suppress_row_hover,
        ))
    })
    .row_height(SAMPLE_BROWSER_ROW_HEIGHT)
    .window(window)
    .overscan_px(SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32)
    .on_window_changed(GuiMessage::SampleBrowserWindowChanged)
    .view()
    .id(SAMPLE_BROWSER_LIST_ID)
    .fill()
}

fn empty_sample_browser_rows() -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line(
            "No audio files in selected folder",
            SAMPLE_BROWSER_ROW_HEIGHT,
        )
        .muted_text(),
        ui::spacer().fill_height(),
    ])
    .spacing(0.0)
    .fill()
}

fn sample_browser_row(row: SampleRowDisplay<'_>) -> ui::View<GuiMessage> {
    let file_id = row.file_id.to_string();
    let hit_target = sample_file_hit_target(
        row.file_id,
        row.selected,
        row.drag_revision,
        row.drag_active,
        row.drag_source,
        row.cached,
        file_id,
        row.suppress_row_hover,
    );
    let row = ui::input_underlay(
        ui::compact_details_row(row.columns.into_iter().map(sample_column_cell)),
        hit_target,
    )
    .key(format!("sample-row-{}", row.file_id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

fn sample_file_hit_target(
    file_id: &str,
    selected: bool,
    drag_revision: u64,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
    hit_path: String,
    suppress_hover: bool,
) -> ui::View<GuiMessage> {
    ui::custom_widget_direct(SampleFileHitTarget::new(
        hit_path,
        selected,
        drag_active,
        drag_source,
        cached,
        suppress_hover,
    ))
    .key(format!("sample-row-hit-{file_id}-{drag_revision}"))
    .fill_width()
    .height(22.0)
}

fn sample_column_cell(column: SampleColumnDisplay<'_>) -> ui::View<GuiMessage> {
    match column.content {
        SampleColumnContent::Text { value, cached } => {
            sample_file_cell(value, column.width, column.file_id, column.id, cached)
        }
        SampleColumnContent::Rename(rename) => {
            sample_rename_cell(rename, column.width, column.file_id)
        }
        SampleColumnContent::Rating(indicator) => {
            sample_rating_cell(indicator, column.width, column.file_id)
        }
        SampleColumnContent::Collection(colors) => {
            sample_collection_cell(colors, column.width, column.file_id)
        }
    }
}

fn sample_rename_cell(rename: FileRenameView, width: f32, file_id: &str) -> ui::View<GuiMessage> {
    ui::compact_details_cell(
        ui::text_input(rename.draft)
            .selection(rename.selection_start, rename.selection_end)
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
            })
            .id(rename.input_id)
            .key(format!("sample-rename-input-{file_id}")),
        Some(width),
    )
}

fn sample_collection_cell(
    colors: Vec<ui::Rgba8>,
    width: f32,
    file_id: &str,
) -> ui::View<GuiMessage> {
    ui::compact_details_cell(
        ui::marker_run_colors(colors)
            .side(6)
            .gap(4)
            .inset(4)
            .view()
            .key(format!("sample-collection-{file_id}")),
        Some(width),
    )
}

fn sample_rating_cell(
    indicator: RatingIndicator,
    width: f32,
    file_id: &str,
) -> ui::View<GuiMessage> {
    if indicator.shows_keep_badge() {
        return ui::compact_details_anchored_cell_from_parts(
            ui::CompactDetailsAnchoredCellParts::new(
                ui::passive_badge("KEEP").style(ui::WidgetStyle::subtle(ui::WidgetTone::Warning)),
                ui::Vector2::new(38.0, 14.0),
            )
            .width(Some(width))
            .horizontal(ui::LayerHorizontalAnchor::End)
            .vertical(ui::LayerVerticalAnchor::Start)
            .inset(2.0, 3.0),
        )
        .key(format!("sample-rating-{file_id}"));
    }

    ui::compact_details_cell(
        ui::marker_run(indicator.color(), indicator.count() as u8)
            .side(5)
            .gap(4)
            .inset(4)
            .view()
            .key(format!("sample-rating-{file_id}")),
        Some(width),
    )
}

fn sample_file_cell(
    value: String,
    width: f32,
    file_id: &str,
    column_id: &str,
    cached: bool,
) -> ui::View<GuiMessage> {
    let text = ui::text(value);
    let text = if cached { text } else { text.muted_text() };
    ui::compact_details_cell(
        text.key(format!("sample-{file_id}-{column_id}")),
        Some(width),
    )
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
