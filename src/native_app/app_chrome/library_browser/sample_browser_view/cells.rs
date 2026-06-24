use radiant::prelude as ui;

use super::identity;
use super::row_projection::{SampleColumnContent, SampleColumnDisplay};
use super::row_widgets::RatingIndicator;
use super::similarity_aspect_color;
use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::folder_browser::commands::FileRenameView;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::SimilarityAspectStrengths;

const SIMILARITY_TOGGLE_WIDTH: f32 = 22.0;
const SIMILARITY_TOGGLE_SIZE: f32 = 18.0;
const SIMILARITY_SCORE_TRACK: ui::Rgba8 = ui::Rgba8::new(64, 68, 72, 150);
pub(super) const SIMILARITY_SCORE_FILL: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 230);
const SIMILARITY_ASPECT_TRACK: ui::Rgba8 = ui::Rgba8::new(38, 42, 46, 190);
pub(super) const SIMILARITY_ASPECT_DISABLED_TRACK: ui::Rgba8 = ui::Rgba8::new(24, 26, 28, 210);
const SIMILARITY_ASPECT_WIDTH: f32 = 14.0;
const SIMILARITY_ANCHOR_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    ui::Rgba8::new(238, 238, 238, 220),
    ui::Rgba8::new(255, 160, 82, 255),
    ui::Rgba8::new(142, 146, 150, 210),
);

pub(super) fn sample_column_cell(column: SampleColumnDisplay<'_>) -> ui::View<GuiMessage> {
    match column.content {
        SampleColumnContent::Text { value, cached } => {
            sample_file_cell(value, column.width, column.file_id, column.id, cached)
        }
        SampleColumnContent::Rename(rename) => sample_rename_cell(rename, column.width),
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

pub(super) fn similarity_anchor_toggle(
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
        .key(identity::similarity_anchor_key(&file_id))
        .size(SIMILARITY_TOGGLE_WIDTH, SIMILARITY_TOGGLE_SIZE);
    button.tooltip_if(
        help_tooltips_enabled,
        "Similarity anchor: compare nearby samples against this one.",
    )
}

pub(super) fn sample_playback_type_cell(
    label: Option<&'static str>,
    width: f32,
    file_id: &str,
) -> ui::View<GuiMessage> {
    let text = label.unwrap_or("-");
    let text = ui::text(text)
        .key(identity::playback_type_key(file_id))
        .height(18.0)
        .fill_width();
    let text = if label.is_some() {
        text
    } else {
        text.muted_text()
    };
    ui::compact_details_cell(text, Some(width))
}

fn sample_rename_cell(rename: FileRenameView, width: f32) -> ui::View<GuiMessage> {
    ui::compact_details_cell(
        ui::text_input(rename.draft)
            .selection(rename.selection_start, rename.selection_end)
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
            })
            .id(rename.input_id),
        Some(width),
    )
}

pub(super) fn sample_collection_cell(
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
            .key(identity::collection_key(file_id)),
        Some(width),
    )
}

pub(super) fn sample_similarity_cell(
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
                .key(identity::similarity_score_key(file_id))
                .height(12.0)
                .fill_width(),
        );
        ui::row(cells).spacing(3.0).height(18.0).fill_width()
    } else {
        ui::text("N/A")
            .muted_text()
            .key(identity::missing_similarity_score_key(file_id))
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
        .key(identity::similarity_aspect_key(aspect, file_id))
        .height(12.0)
        .width(SIMILARITY_ASPECT_WIDTH)
}

fn similarity_anchor_icon(active: bool, available: bool) -> ui::SvgIcon {
    SIMILARITY_ANCHOR_ICON.icon_for_state(SIMILARITY_ANCHOR_ICON_TINTS, available, active)
}

pub(super) fn sample_rating_cell(
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
        .key(identity::rating_key(file_id));
    }

    ui::compact_details_cell(
        ui::marker_run(indicator.color(), indicator.count() as u8)
            .side(5)
            .gap(4)
            .inset(4)
            .view()
            .key(identity::rating_key(file_id)),
        Some(width),
    )
}

pub(super) fn sample_file_cell(
    value: String,
    width: f32,
    file_id: &str,
    column_id: &str,
    _cached: bool,
) -> ui::View<GuiMessage> {
    let text = ui::text(value);
    ui::compact_details_cell(
        text.key(identity::text_cell_key(file_id, column_id)),
        Some(width),
    )
}

static SIMILARITY_ANCHOR_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="8" cy="8" r="4.2" fill="currentColor"/>
</svg>"#,
);
