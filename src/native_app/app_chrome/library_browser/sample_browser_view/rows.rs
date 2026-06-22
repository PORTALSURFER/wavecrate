use radiant::prelude as ui;
use std::collections::HashMap;

use super::row_projection::{
    SampleColumnContent, SampleColumnDisplay, SampleRowDisplay, sample_row_display,
};
use super::row_widgets::RatingIndicator;
use super::{SampleFileHitTarget, similarity_aspect_color};
use crate::native_app::app::{GuiMessage, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::commands::FileRenameView;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::SimilarityAspectStrengths;
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleList;
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
};

const SIMILARITY_TOGGLE_WIDTH: f32 = 22.0;
const SIMILARITY_TOGGLE_SIZE: f32 = 18.0;
const SIMILARITY_SCORE_TRACK: ui::Rgba8 = ui::Rgba8::new(64, 68, 72, 150);
const SIMILARITY_SCORE_FILL: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 230);
const SIMILARITY_ASPECT_TRACK: ui::Rgba8 = ui::Rgba8::new(38, 42, 46, 190);
const SIMILARITY_ASPECT_DISABLED_TRACK: ui::Rgba8 = ui::Rgba8::new(24, 26, 28, 210);
const SIMILARITY_ASPECT_WIDTH: f32 = 14.0;

pub(super) fn sample_browser_rows(
    visible_samples: &VisibleSampleList<'_>,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    if visible_samples.total_count == 0 {
        return empty_sample_browser_rows();
    }

    ui::virtual_list_windowed(|index: usize| {
        let Some(row_index) = index.checked_sub(visible_samples.window.window_start) else {
            return ui::empty().fill_width().height(SAMPLE_BROWSER_ROW_HEIGHT);
        };
        let Some(row) = visible_samples.rows.get(row_index) else {
            return ui::empty().fill_width().height(SAMPLE_BROWSER_ROW_HEIGHT);
        };
        sample_browser_row(
            sample_row_display(
                row,
                &visible_samples.columns,
                visible_samples.similarity_mode_active,
                visible_samples.similarity_controls.aspect_enabled_flags(),
                name_view_mode,
                metadata_tags_by_file,
            ),
            help_tooltips_enabled,
        )
    })
    .row_height(SAMPLE_BROWSER_ROW_HEIGHT)
    .window(visible_samples.window)
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

fn sample_browser_row(
    row: SampleRowDisplay<'_>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let file_id = row.file_id.to_string();
    let file_id_for_toggle = row.file_id.to_string();
    let hit_target = sample_file_hit_target(SampleFileHitTargetModel {
        file_id: row.file_id,
        selected: row.selected,
        copy_flash: row.copy_flash,
        drag_revision: row.drag_revision,
        drag_active: row.drag_active,
        drag_source: row.drag_source,
        cached: row.cached,
        missing: row.missing,
        hit_path: file_id,
        help_tooltips_enabled,
    });
    let row = ui::input_underlay(
        ui::row([
            similarity_anchor_toggle(
                file_id_for_toggle,
                row.similarity_anchor,
                row.similarity_strength,
                help_tooltips_enabled,
            ),
            ui::compact_details_row(row.columns.into_iter().map(sample_column_cell)).fill_width(),
        ])
        .spacing(0.0)
        .fill_width()
        .height(SAMPLE_BROWSER_ROW_HEIGHT),
        hit_target,
    )
    .key(format!("sample-row-{}", row.file_id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

struct SampleFileHitTargetModel<'a> {
    file_id: &'a str,
    selected: bool,
    copy_flash: bool,
    drag_revision: u64,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
    missing: bool,
    hit_path: String,
    help_tooltips_enabled: bool,
}

fn sample_file_hit_target(model: SampleFileHitTargetModel<'_>) -> ui::View<GuiMessage> {
    let target = ui::custom_widget_direct(SampleFileHitTarget::new(
        model.hit_path,
        model.selected,
        model.copy_flash,
        model.drag_active,
        model.drag_source,
        model.cached,
        model.missing,
    ))
    .key(format!(
        "sample-row-hit-{}-{}",
        model.file_id, model.drag_revision
    ))
    .fill_width()
    .height(22.0);
    target.tooltip_opt(model.help_tooltips_enabled.then_some(
        "Sample row: select, double-click to load, drag to copy, right-click for actions.",
    ))
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
        SampleColumnContent::PlaybackType(label) => {
            sample_playback_type_cell(label, column.width, column.file_id)
        }
        SampleColumnContent::Collection(colors) => {
            sample_collection_cell(colors, column.width, column.file_id)
        }
        SampleColumnContent::Similarity {
            overall,
            aspects,
            aspect_enabled,
        } => sample_similarity_cell(
            overall,
            aspects,
            aspect_enabled,
            column.width,
            column.file_id,
        ),
    }
}

fn similarity_anchor_toggle(
    file_id: String,
    active: bool,
    strength: Option<f32>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let button = ui::icon_button(similarity_anchor_icon(active, strength.is_some()))
        .subtle()
        .active(active)
        .message(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ToggleSimilarityAnchor(file_id.clone()),
        ))
        .key(format!("sample-similarity-anchor-{file_id}"))
        .size(SIMILARITY_TOGGLE_WIDTH, SIMILARITY_TOGGLE_SIZE);
    button.tooltip_opt(
        help_tooltips_enabled
            .then_some("Similarity anchor: compare nearby samples against this one."),
    )
}

fn sample_playback_type_cell(
    label: Option<&'static str>,
    width: f32,
    file_id: &str,
) -> ui::View<GuiMessage> {
    let text = label.unwrap_or("-");
    let text = ui::text(text)
        .key(format!("sample-playback-type-{file_id}"))
        .height(18.0)
        .fill_width();
    let text = if label.is_some() {
        text
    } else {
        text.muted_text()
    };
    ui::compact_details_cell(text, Some(width))
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

fn sample_similarity_cell(
    overall: Option<f32>,
    aspects: SimilarityAspectStrengths,
    aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    width: f32,
    file_id: &str,
) -> ui::View<GuiMessage> {
    let content = if let Some(overall) = overall {
        let mut cells = Vec::with_capacity(wavecrate_analysis::aspects::ASPECT_COUNT + 1);
        for aspect in wavecrate_analysis::aspects::SimilarityAspect::ORDER {
            cells.push(sample_similarity_aspect_indicator(
                aspect,
                aspects[aspect.index()],
                aspect_enabled[aspect.index()],
                file_id,
            ));
        }
        cells.push(
            ui::determinate_progress_bar(overall)
                .colors(SIMILARITY_SCORE_TRACK, SIMILARITY_SCORE_FILL)
                .max_track_height(5.0)
                .mapped(|_| GuiMessage::CloseContextMenu)
                .key(format!("sample-similarity-score-{file_id}"))
                .height(12.0)
                .fill_width(),
        );
        ui::row(cells).spacing(3.0).height(18.0).fill_width()
    } else {
        ui::text("N/A")
            .muted_text()
            .key(format!("sample-similarity-score-missing-{file_id}"))
            .height(18.0)
            .fill_width()
    };
    ui::compact_details_cell(content, Some(width))
}

fn sample_similarity_aspect_indicator(
    aspect: wavecrate_analysis::aspects::SimilarityAspect,
    strength: Option<f32>,
    enabled: bool,
    file_id: &str,
) -> ui::View<GuiMessage> {
    let (track, fill, value) = if enabled {
        let fill = if strength.is_some() {
            similarity_aspect_color(aspect)
        } else {
            SIMILARITY_ASPECT_TRACK
        };
        (SIMILARITY_ASPECT_TRACK, fill, strength.unwrap_or(0.0))
    } else {
        (
            SIMILARITY_ASPECT_DISABLED_TRACK,
            SIMILARITY_ASPECT_DISABLED_TRACK,
            0.0,
        )
    };
    ui::determinate_progress_bar(value)
        .colors(track, fill)
        .max_track_height(10.0)
        .mapped(|_| GuiMessage::CloseContextMenu)
        .key(format!(
            "sample-similarity-aspect-{}-{file_id}",
            aspect.index()
        ))
        .height(12.0)
        .width(SIMILARITY_ASPECT_WIDTH)
}

fn similarity_anchor_icon(active: bool, available: bool) -> ui::SvgIcon {
    let color = if active {
        ui::Rgba8::new(255, 160, 82, 255)
    } else if available {
        ui::Rgba8::new(238, 238, 238, 220)
    } else {
        ui::Rgba8::new(142, 146, 150, 210)
    };
    SIMILARITY_ANCHOR_ICON.icon(color)
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
    _cached: bool,
) -> ui::View<GuiMessage> {
    let text = ui::text(value);
    ui::compact_details_cell(
        text.key(format!("sample-{file_id}-{column_id}")),
        Some(width),
    )
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;

static SIMILARITY_ANCHOR_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="8" cy="8" r="4.2" fill="currentColor"/>
</svg>"#,
);
