use radiant::prelude as ui;

use self::projection::{
    RatingCellProjection, SampleCellContentProjection, SampleCellProjection,
    SimilarityAspectProjection, SimilarityCellProjection, sample_cell_projection,
};
use super::identity;
use super::row_projection::SampleColumnDisplay;
#[cfg(test)]
use super::row_widgets::RatingIndicator;
use super::similarity_aspect_color;
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::palette::{ACCENT, ACCENT_SOFT, TEXT_MUTED, TEXT_PRIMARY};
use crate::native_app::sample_library::folder_browser::commands::FileRenameView;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
#[cfg(test)]
use crate::native_app::sample_library::folder_browser::model::SimilarityAspectStrengths;

mod projection;

const SIMILARITY_TOGGLE_WIDTH: f32 = 22.0;
const SIMILARITY_TOGGLE_SIZE: f32 = 18.0;
const SIMILARITY_SCORE_TRACK: ui::Rgba8 = ui::Rgba8::new(48, 50, 50, 150);
pub(super) const SIMILARITY_SCORE_FILL: ui::Rgba8 = ACCENT.with_alpha(230);
const SIMILARITY_ASPECT_TRACK: ui::Rgba8 = ui::Rgba8::new(40, 43, 44, 190);
pub(super) const SIMILARITY_ASPECT_DISABLED_TRACK: ui::Rgba8 = ui::Rgba8::new(24, 27, 28, 210);
const SIMILARITY_ASPECT_WIDTH: f32 = 14.0;
const SAMPLE_NAME_BADGE_LIMIT: usize = 4;
pub(super) const RATING_MARKER_SIDE: u8 = 5;
pub(super) const LOCKED_KEEP_RATING_MARKER_SIDE: u8 = 7;
pub(super) const LOCKED_KEEP_RATING_COLOR: ui::Rgba8 = ui::Rgba8::new(232, 188, 56, 245);
/// Keeps compact column content left of the header resize-divider gutter.
pub(super) const COMPACT_COLUMN_CONTENT_TRAILING_GUTTER: f32 = 14.0;
const SIMILARITY_ANCHOR_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    TEXT_PRIMARY.with_alpha(220),
    ACCENT,
    TEXT_MUTED.with_alpha(210),
);

/// Render one projected sample-browser cell.
pub(super) fn sample_column_cell(
    column: SampleColumnDisplay,
    selected_name: bool,
) -> ui::View<GuiMessage> {
    render_sample_cell(sample_cell_projection(column), selected_name)
}

fn render_sample_cell(
    projection: SampleCellProjection,
    selected_name: bool,
) -> ui::View<GuiMessage> {
    match projection.content {
        SampleCellContentProjection::Name {
            text,
            badges,
            muted,
        } => sample_name_cell(text, badges, muted, selected_name, projection.width),
        SampleCellContentProjection::Curation { badges, muted } => {
            sample_badge_run_cell(badges, muted, projection.width)
        }
        SampleCellContentProjection::Harvest { badges, muted } => {
            sample_badge_run_cell(badges, muted, projection.width)
        }
        SampleCellContentProjection::Text { value, muted } => {
            sample_file_cell_with_tone(value, projection.width, muted)
        }
        SampleCellContentProjection::Rename(rename) => sample_rename_cell(rename, projection.width),
        SampleCellContentProjection::Rating(rating) => render_rating_cell(rating, projection.width),
        SampleCellContentProjection::PlaybackType(playback_type) => render_playback_type_cell(
            playback_type.label,
            playback_type.available,
            projection.width,
        ),
        SampleCellContentProjection::Collection(colors) => {
            sample_collection_cell(colors, projection.width)
        }
        SampleCellContentProjection::Similarity(similarity) => {
            render_similarity_cell(similarity, projection.width)
        }
    }
}

pub(super) fn similarity_anchor_toggle(
    file_id: String,
    active: bool,
    strength: Option<f32>,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let available = strength.is_some();
    let button = ui::icon_button(similarity_anchor_icon(active, available))
        // The anchor occupies the leading sample gutter, but must not paint a
        // second boxed column edge beside the pane resize divider.
        .bare()
        // The sample row remains the keyboard-navigation owner. The nested
        // sphere is a pointer toggle and must not replace row focus state.
        .focus(ui::FocusBehavior::Pointer)
        .hover_icon(SIMILARITY_ANCHOR_ICON.icon(if active {
            ACCENT_SOFT
        } else if available {
            ACCENT
        } else {
            TEXT_PRIMARY
        }))
        .message(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ToggleSimilarityAnchor(file_id.clone()),
        ))
        .key(identity::RETAINED_SIMILARITY_ANCHOR_BUTTON_KEY)
        .size(SIMILARITY_TOGGLE_WIDTH, SIMILARITY_TOGGLE_SIZE);
    button.tooltip_if(
        help_tooltips_enabled,
        "Similarity anchor: compare nearby samples against this one.",
    )
}

#[cfg(test)]
pub(super) fn sample_playback_type_cell(
    label: Option<&'static str>,
    width: f32,
) -> ui::View<GuiMessage> {
    let projection = SampleCellProjection::playback_type(width, label);
    let SampleCellContentProjection::PlaybackType(playback_type) = projection.content else {
        unreachable!("playback type constructor should project playback type content");
    };
    render_playback_type_cell(playback_type.label, playback_type.available, width)
}

/// Render the passive playback-type cell.
fn render_playback_type_cell(label: String, available: bool, width: f32) -> ui::View<GuiMessage> {
    let text = compact_text(label);
    let text = if available { text } else { text.muted_text() };
    compact_column_content_cell(text, width)
}

fn sample_rename_cell(rename: FileRenameView, width: f32) -> ui::View<GuiMessage> {
    radiant::application::compact_details_cell(
        ui::text_input(rename.draft)
            .selection(rename.selection_start, rename.selection_end)
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
            })
            .id(rename.input_id),
        Some(width),
    )
}

/// Render the passive collection-membership marker cell.
pub(super) fn sample_collection_cell(colors: Vec<ui::Rgba8>, width: f32) -> ui::View<GuiMessage> {
    compact_column_content_cell(
        ui::marker_run_colors(colors).side(6).gap(4).inset(0).view(),
        width,
    )
}

#[cfg(test)]
pub(super) fn sample_similarity_cell(
    overall: Option<f32>,
    aspects: SimilarityAspectStrengths,
    aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    width: f32,
) -> ui::View<GuiMessage> {
    render_similarity_cell(
        SimilarityCellProjection::new(overall, aspects, aspect_enabled),
        width,
    )
}

fn render_similarity_cell(
    projection: SimilarityCellProjection,
    width: f32,
) -> ui::View<GuiMessage> {
    let content = if let Some(overall) = projection.overall {
        let mut cells = Vec::with_capacity(wavecrate_analysis::aspects::ASPECT_COUNT + 1);
        for aspect in projection.aspects {
            cells.push(sample_similarity_aspect_indicator(aspect));
        }
        cells.push(
            ui::determinate_progress_bar(overall)
                .colors(SIMILARITY_SCORE_TRACK, SIMILARITY_SCORE_FILL)
                .max_track_height(5.0)
                .passive::<GuiMessage>()
                .height(12.0)
                .fill_width(),
        );
        ui::row(cells).spacing(3.0).height(18.0).fill_width()
    } else {
        ui::text("N/A").muted_text().height(18.0).fill_width()
    };
    radiant::application::compact_details_cell(content, Some(width))
}

/// Render one passive similarity aspect indicator.
fn sample_similarity_aspect_indicator(aspect: SimilarityAspectProjection) -> ui::View<GuiMessage> {
    let (track, fill, value) = if aspect.enabled {
        let fill = if aspect.strength.is_some() {
            similarity_aspect_color(aspect.aspect)
        } else {
            SIMILARITY_ASPECT_TRACK
        };
        (
            SIMILARITY_ASPECT_TRACK,
            fill,
            aspect.strength.unwrap_or(0.0),
        )
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
        .passive::<GuiMessage>()
        .height(12.0)
        .width(SIMILARITY_ASPECT_WIDTH)
}

fn similarity_anchor_icon(active: bool, available: bool) -> ui::SvgIcon {
    SIMILARITY_ANCHOR_ICON.icon_for_state(SIMILARITY_ANCHOR_ICON_TINTS, available, active)
}

#[cfg(test)]
/// Render the passive rating cell for focused row rendering tests.
pub(super) fn sample_rating_cell(indicator: RatingIndicator, width: f32) -> ui::View<GuiMessage> {
    render_rating_cell(RatingCellProjection::from_indicator(indicator), width)
}

#[cfg(test)]
pub(super) fn muted_sample_file_cell(value: String, width: f32) -> ui::View<GuiMessage> {
    sample_file_cell_with_tone(value, width, true)
}

/// Render a projected passive rating cell.
fn render_rating_cell(projection: RatingCellProjection, width: f32) -> ui::View<GuiMessage> {
    let visual_key = projection.retained_visual_key();
    if projection == RatingCellProjection::LockedKeepMarker {
        return compact_column_content_cell(
            ui::marker_run(Some(LOCKED_KEEP_RATING_COLOR), 1)
                .side(LOCKED_KEEP_RATING_MARKER_SIDE)
                .gap(4)
                .inset(0)
                .view(),
            width,
        )
        .key(visual_key);
    }

    compact_column_content_cell(
        ui::marker_run(projection.marker_color(), projection.marker_count())
            .side(RATING_MARKER_SIDE)
            .gap(4)
            .inset(0)
            .view(),
        width,
    )
    .key(visual_key)
}

/// Render a passive text cell for a sample file column.
#[cfg(test)]
pub(super) fn sample_file_cell(value: String, width: f32) -> ui::View<GuiMessage> {
    sample_file_cell_with_tone(value, width, false)
}

fn sample_file_cell_with_tone(value: String, width: f32, muted: bool) -> ui::View<GuiMessage> {
    let text = compact_text(value);
    let text = if muted { text.muted_text() } else { text };
    compact_column_content_cell(text, width)
}

fn sample_name_cell(
    text: String,
    badges: Vec<String>,
    muted: bool,
    selected: bool,
    width: f32,
) -> ui::View<GuiMessage> {
    if badges.is_empty() {
        let name = selected_sample_name_text(compact_text(text), muted, selected);
        return compact_column_content_cell(name, width);
    }
    let mut cells = Vec::with_capacity(badges.len() + 1);
    let name = selected_sample_name_text(compact_text(text).fill_width(), muted, selected);
    cells.push(name);
    cells.extend(sample_badge_views(badges));
    compact_column_content_cell(ui::row(cells).spacing(4.0).fill_width(), width)
}

fn selected_sample_name_text(
    name: ui::View<GuiMessage>,
    muted: bool,
    selected: bool,
) -> ui::View<GuiMessage> {
    if selected {
        name.text_color(ui::TextColorRole::Custom(ACCENT))
    } else if muted {
        name.muted_text()
    } else {
        name
    }
}

#[cfg(test)]
pub(super) fn selected_sample_name_cell_for_tests(
    value: String,
    width: f32,
) -> ui::View<GuiMessage> {
    sample_name_cell(value, Vec::new(), false, true, width)
}

fn sample_badge_run_cell(badges: Vec<String>, muted: bool, width: f32) -> ui::View<GuiMessage> {
    if badges.is_empty() {
        return sample_file_cell_with_tone(String::new(), width, muted);
    }
    compact_column_content_cell(
        ui::row(sample_badge_views(badges))
            .spacing(4.0)
            .fill_width(),
        width,
    )
}

fn sample_badge_views(badges: Vec<String>) -> Vec<ui::View<GuiMessage>> {
    let badge_count = badges.len();
    let mut views = badges
        .into_iter()
        .take(SAMPLE_NAME_BADGE_LIMIT)
        .map(sample_passive_badge)
        .collect::<Vec<_>>();
    if badge_count > SAMPLE_NAME_BADGE_LIMIT {
        views.push(sample_passive_badge(format!(
            "+{}",
            badge_count - SAMPLE_NAME_BADGE_LIMIT
        )));
    }
    views
}

fn sample_passive_badge(label: String) -> ui::View<GuiMessage> {
    ui::passive_badge(label)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
        .height(14.0)
}

fn compact_text(text: impl Into<ui::TextContent>) -> ui::View<GuiMessage> {
    ui::text(text).height(18.0).truncate()
}

fn compact_column_content_cell(content: ui::View<GuiMessage>, width: f32) -> ui::View<GuiMessage> {
    let content_width = (width - COMPACT_COLUMN_CONTENT_TRAILING_GUTTER).max(0.0);
    radiant::application::compact_details_cell(
        ui::row([content.width(content_width).height(20.0)])
            .spacing(0.0)
            .height(20.0),
        Some(width),
    )
}

#[cfg(test)]
pub(super) fn sample_harvest_badge_cell(badges: Vec<String>, width: f32) -> ui::View<GuiMessage> {
    sample_badge_run_cell(badges, false, width)
}

static SIMILARITY_ANCHOR_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="8" cy="8" r="4.2" fill="currentColor"/>
</svg>"#,
);
