use radiant::prelude as ui;
use std::collections::HashMap;

use super::cells::{sample_column_cell, similarity_anchor_toggle};
use super::hit_target::{SampleFileHitTargetModel, sample_file_hit_target};
use super::row_projection::{SampleRowDisplay, sample_row_display};
use crate::native_app::app::{GuiMessage, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleList;
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
};

pub(super) fn sample_browser_rows(
    visible_samples: &VisibleSampleList<'_>,
    name_view_mode: SampleNameViewMode,
    curation_mode_enabled: bool,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cut_file_ids: Option<&[String]>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    if visible_samples.total_count == 0 {
        return empty_sample_browser_rows(curation_mode_enabled);
    }

    ui::virtual_list_materialized_windowed(
        visible_samples.window,
        &visible_samples.rows,
        |_, row| {
            sample_browser_row(
                sample_row_display(
                    row,
                    &visible_samples.columns,
                    visible_samples.similarity_mode_active,
                    visible_samples.similarity_controls.aspect_enabled_flags(),
                    name_view_mode,
                    metadata_tags_by_file,
                    cut_file_ids,
                ),
                help_tooltips_enabled,
            )
        },
    )
    .row_height(SAMPLE_BROWSER_ROW_HEIGHT)
    .overscan_px(SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32)
    .on_window_changed(GuiMessage::SampleBrowserWindowChanged)
    .view()
    .id(SAMPLE_BROWSER_LIST_ID)
    .fill()
}

fn empty_sample_browser_rows(curation_mode_enabled: bool) -> ui::View<GuiMessage> {
    let message = if curation_mode_enabled {
        "No files left to curate"
    } else {
        "No audio files in selected folder"
    };
    ui::column([
        ui::text_line(message, SAMPLE_BROWSER_ROW_HEIGHT).muted_text(),
        ui::spacer().fill_height(),
    ])
    .spacing(0.0)
    .fill()
}

fn sample_browser_row(
    row: SampleRowDisplay<'_>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let file_id = row.file_id.to_string();
    let file_id_for_toggle = row.file_id.to_string();
    let row_content = ui::row([
        similarity_anchor_toggle(
            file_id_for_toggle,
            row.similarity_anchor,
            row.similarity_strength,
            help_tooltips_enabled,
        ),
        radiant::application::compact_details_row(row.columns.into_iter().map(sample_column_cell))
            .fill_width(),
    ])
    .spacing(0.0)
    .fill_width()
    .height(SAMPLE_BROWSER_ROW_HEIGHT);
    let row = sample_file_hit_target(
        row_content,
        SampleFileHitTargetModel {
            file_id: row.file_id,
            explicitly_selected: row.explicitly_selected,
            focused: row.focused,
            copy_flash: row.copy_flash,
            protected_source_error_flash: row.protected_source_error_flash,
            cut_pending: row.cut_pending,
            drag_active: row.drag_active,
            drag_source: row.drag_source,
            cached: row.cached,
            missing: row.missing,
            hit_path: file_id,
            help_tooltips_enabled,
        },
    )
    .fill_width()
    .height(SAMPLE_BROWSER_ROW_HEIGHT);
    row.style(ui::WidgetStyle::default())
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
