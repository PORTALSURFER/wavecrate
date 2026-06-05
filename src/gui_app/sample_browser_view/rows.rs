use radiant::prelude as ui;
use std::collections::HashMap;
use std::collections::HashSet;

use super::SampleFileHitTarget;
use super::row_widgets::RatingIndicator;
use crate::gui_app::{
    GuiMessage, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
    SampleNameViewMode,
    folder_browser::{self, FileColumn, FileEntry, FolderBrowserMessage, FolderBrowserState},
};

pub(super) fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
    columns: &[&FileColumn],
    window: ui::VirtualListWindow,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached_sample_paths: &HashSet<String>,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return empty_sample_browser_rows();
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
                folder_browser.drag_revision(),
                folder_browser.file_drag_active(),
                folder_browser.file_drag_source(&file.id),
                folder_browser,
                columns,
                name_view_mode,
                metadata_tags_by_file,
                cached_sample_paths.contains(&file.id),
                suppress_row_hover,
            )
        },
        SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32,
    )
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

fn sample_browser_row(
    file: &FileEntry,
    selected: bool,
    rename: Option<folder_browser::FileRenameView>,
    drag_revision: u64,
    drag_active: bool,
    drag_source: bool,
    folder_browser: &FolderBrowserState,
    columns: &[&FileColumn],
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached: bool,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    let hit_path = file.id.clone();
    let hit_target = sample_file_hit_target(
        file,
        selected,
        drag_revision,
        drag_active,
        drag_source,
        cached,
        hit_path,
        suppress_row_hover,
    );
    let row = ui::input_underlay(
        ui::compact_details_row(columns.iter().map(|column| {
            sample_column_cell(
                file,
                rename.clone(),
                column,
                folder_browser,
                name_view_mode,
                metadata_tags_by_file,
                cached,
            )
        })),
        hit_target,
    )
    .key(format!("sample-row-{}", file.id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

fn sample_file_hit_target(
    file: &FileEntry,
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
    .key(format!("sample-row-hit-{}-{drag_revision}", file.id))
    .fill_width()
    .height(22.0)
}

fn sample_column_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    column: &FileColumn,
    folder_browser: &FolderBrowserState,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached: bool,
) -> ui::View<GuiMessage> {
    if column.id == "name" {
        return sample_name_cell(
            file,
            rename,
            column.width,
            name_view_mode,
            metadata_tags_by_file,
            cached,
        );
    }
    if column.id == "rating" {
        return sample_rating_cell(file, column.width);
    }
    if column.id == "collection" {
        return sample_collection_cell(file, column.width, folder_browser);
    }
    sample_file_cell(
        file,
        sample_file_column_value(file, column.id.as_str()),
        column.width,
        column.id.as_str(),
        cached,
    )
}

fn sample_name_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    width: f32,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached: bool,
) -> ui::View<GuiMessage> {
    let Some(rename) = rename else {
        return sample_file_cell(
            file,
            sample_name_cell_value(file, name_view_mode, metadata_tags_by_file),
            width,
            "name",
            cached,
        );
    };
    ui::compact_details_cell(
        ui::text_input(rename.draft)
            .selection(rename.selection_start, rename.selection_end)
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
            })
            .id(rename.input_id)
            .key(format!("sample-rename-input-{}", file.id)),
        Some(width),
    )
}

pub(super) fn sample_name_cell_value(
    file: &FileEntry,
    mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> String {
    match mode {
        SampleNameViewMode::DiskFilename => file.stem.clone(),
        SampleNameViewMode::MetadataLabel => {
            metadata_display_stem(file, metadata_tags_by_file.get(&file.id).map(Vec::as_slice))
        }
    }
}

fn metadata_display_stem(file: &FileEntry, metadata_tags: Option<&[String]>) -> String {
    let display = metadata_tags
        .unwrap_or(&[])
        .iter()
        .filter(|tag| !tag.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("_");
    if display.is_empty() {
        file.stem.clone()
    } else {
        display
    }
}

fn sample_file_column_value(file: &FileEntry, column_id: &str) -> String {
    match column_id {
        "extension" => file.extension.clone(),
        "size" => file.size.clone(),
        "modified" => file.modified.clone(),
        "kind" => file.kind.clone(),
        "collection" => file
            .collection_memberships()
            .into_iter()
            .map(folder_browser::collection_hotkey)
            .map(|hotkey| hotkey.to_string())
            .collect::<Vec<_>>()
            .join(","),
        "path" => file.id.clone(),
        _ => file.stem.clone(),
    }
}

fn sample_collection_cell(
    file: &FileEntry,
    width: f32,
    folder_browser: &FolderBrowserState,
) -> ui::View<GuiMessage> {
    let colors = file
        .collection_memberships()
        .into_iter()
        .filter_map(|collection| folder_browser.collection_color(collection));

    ui::compact_details_cell(
        ui::marker_run_colors(colors)
            .side(6)
            .gap(4)
            .inset(4)
            .view()
            .key(format!("sample-collection-{}", file.id)),
        Some(width),
    )
}

fn sample_rating_cell(file: &FileEntry, width: f32) -> ui::View<GuiMessage> {
    let indicator = RatingIndicator::new(file.rating, file.rating_locked);
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
        .key(format!("sample-rating-{}", file.id));
    }

    ui::compact_details_cell(
        ui::marker_run(indicator.color(), indicator.count() as u8)
            .side(5)
            .gap(4)
            .inset(4)
            .view()
            .key(format!("sample-rating-{}", file.id)),
        Some(width),
    )
}

fn sample_file_cell(
    file: &FileEntry,
    value: String,
    width: f32,
    column_id: &str,
    cached: bool,
) -> ui::View<GuiMessage> {
    let text = ui::text(value);
    let text = if cached { text } else { text.muted_text() };
    ui::compact_details_cell(
        text.key(format!("sample-{}-{column_id}", file.id)),
        Some(width),
    )
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
