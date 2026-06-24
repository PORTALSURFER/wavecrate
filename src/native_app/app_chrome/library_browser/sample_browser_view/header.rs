use radiant::prelude as ui;

mod projection;
#[cfg(test)]
mod tests;

use crate::native_app::app::{GuiMessage, MetadataMessage, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::FileColumn;
use crate::native_app::sample_library::folder_browser::projection::FileColumnDragFeedback;
use wavecrate::sample_sources::config::SimilarityAspectSettings;

use self::projection::{
    HeaderColumnProjection, RandomNavigationButtonProjection, SampleBrowserHeaderProjection,
    SampleNameViewModeButtonProjection, SampleSimilarityAspectControlProjection,
    SampleSimilarityControlsProjection, SampleSimilarityHeaderProjection,
};
use super::{SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH, identity, similarity_aspect_color};

const SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH: f32 = 22.0;
const SAMPLE_SIMILARITY_ASPECT_HEADER_WIDTH: f32 = 14.0;
const SAMPLE_BROWSER_ICON_ACTIVE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 255);
const SAMPLE_BROWSER_ICON_ENABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(238, 238, 238, 255);
const SAMPLE_BROWSER_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    SAMPLE_BROWSER_ICON_ENABLED_COLOR,
    SAMPLE_BROWSER_ICON_ACTIVE_COLOR,
    SAMPLE_BROWSER_ICON_ENABLED_COLOR,
);

pub(super) struct SampleBrowserHeaderBar<'a> {
    pub(super) columns: &'a [&'a FileColumn],
    pub(super) sort: &'a ui::DetailsSort,
    pub(super) drag_feedback: Option<&'a FileColumnDragFeedback>,
    pub(super) mode: SampleNameViewMode,
    pub(super) random_navigation_enabled: bool,
    pub(super) similarity_mode_active: bool,
    pub(super) similarity_controls: &'a SimilarityAspectSettings,
    pub(super) help_tooltips_enabled: bool,
}

pub(super) fn sample_browser_header_bar(model: SampleBrowserHeaderBar<'_>) -> ui::View<GuiMessage> {
    let projection = SampleBrowserHeaderProjection::from_model(model);
    ui::row([
        sample_browser_header(
            projection.columns,
            projection.sort,
            projection.drag_marker_x,
            projection.similarity_header,
        )
        .fill_width(),
        random_navigation_button(projection.random_navigation).tooltip_if(
            projection.help_tooltips_enabled,
            projection.random_navigation.tooltip,
        ),
        sample_name_view_mode_button(projection.name_view_mode).tooltip_if(
            projection.help_tooltips_enabled,
            projection.name_view_mode.tooltip,
        ),
    ])
    .fill_width()
    .height(24.0)
    .spacing(6.0)
}

pub(super) fn sample_similarity_controls_bar(
    controls: &SimilarityAspectSettings,
) -> ui::View<GuiMessage> {
    let projection = SampleSimilarityControlsProjection::from_settings(controls);
    let mut controls_row = Vec::with_capacity(projection.aspects.len() + 2);
    controls_row.push(
        ui::spacer()
            .width(SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH)
            .height(22.0),
    );
    controls_row.push(
        ui::toggle(projection.weighting_label, projection.weighting_enabled)
            .subtle()
            .message(GuiMessage::SetSimilarityAspectWeightingEnabled)
            .id(identity::sample_similarity_weighting_toggle_id())
            .size(70.0, 20.0),
    );
    for aspect in projection.aspects {
        controls_row.push(sample_similarity_aspect_control(aspect));
    }
    ui::row(controls_row)
        .spacing(5.0)
        .padding_x(3.0)
        .fill_width()
        .height(28.0)
}

fn random_navigation_button(projection: RandomNavigationButtonProjection) -> ui::View<GuiMessage> {
    ui::icon_button(random_navigation_icon(projection.active))
        .active(projection.active)
        .message(GuiMessage::ToggleRandomNavigationMode)
        .id(identity::random_navigation_toggle_id())
        .size(28.0, 22.0)
}

fn random_navigation_icon(active: bool) -> ui::SvgIcon {
    DICE_ICON.icon_for_state(SAMPLE_BROWSER_ICON_TINTS, true, active)
}

fn sample_name_view_mode_button(
    projection: SampleNameViewModeButtonProjection,
) -> ui::View<GuiMessage> {
    ui::button(projection.label)
        .message(GuiMessage::Metadata(
            MetadataMessage::ToggleSampleNameViewMode,
        ))
        .key(identity::SAMPLE_NAME_VIEW_MODE_TOGGLE_KEY)
        .size(58.0, 22.0)
}

static DICE_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="2.75" y="2.75" width="10.5" height="10.5" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <circle cx="5.4" cy="5.4" r="1.15" fill="currentColor"/>
  <circle cx="10.6" cy="5.4" r="1.15" fill="currentColor"/>
  <circle cx="8" cy="8" r="1.15" fill="currentColor"/>
  <circle cx="5.4" cy="10.6" r="1.15" fill="currentColor"/>
  <circle cx="10.6" cy="10.6" r="1.15" fill="currentColor"/>
</svg>"#,
);

fn sample_browser_header(
    columns: Vec<HeaderColumnProjection<'_>>,
    sort: &ui::DetailsSort,
    drag_marker_x: Option<f32>,
    similarity_header: SampleSimilarityHeaderProjection,
) -> ui::View<GuiMessage> {
    let header_cells = columns
        .iter()
        .flat_map(|column| sample_header_cells(column, sort, &similarity_header));
    let header = ui::row([
        ui::spacer()
            .width(SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH)
            .height(24.0),
        ui::compact_details_header_row(header_cells).fill_width(),
    ])
    .spacing(0.0)
    .fill_width()
    .height(24.0);
    let Some(marker_x) = drag_marker_x else {
        return header;
    };
    ui::stack([header, column_drop_marker(marker_x)])
        .fill_width()
        .height(24.0)
}

fn column_drop_marker(x: f32) -> ui::View<GuiMessage> {
    ui::local_drop_marker(x, ui::Rgba8::new(255, 160, 82, 230), 2.0, 20.0)
        .key(identity::SAMPLE_COLUMN_DROP_MARKER_KEY)
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
}

fn sample_header_cell(
    projection: &HeaderColumnProjection<'_>,
    sort: &ui::DetailsSort,
) -> ui::View<GuiMessage> {
    let column = projection.column;
    let sort_id = column.id.clone();
    let drag_id = column.id.clone();
    let resize_id = column.id.clone();
    let label = ui::details_sort_label(column.label.as_str(), column.id.as_str(), Some(sort));
    ui::compact_resizable_details_header_cell(
        label,
        column.width,
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
    .id(identity::sample_header_cell_id(column.id.as_str()))
}

fn sample_header_cells(
    column: &HeaderColumnProjection<'_>,
    sort: &ui::DetailsSort,
    similarity_header: &SampleSimilarityHeaderProjection,
) -> Vec<ui::View<GuiMessage>> {
    let mut cells = vec![sample_header_cell(column, sort)];
    if column.show_similarity_after {
        cells.push(sample_similarity_header_cell(similarity_header));
    }
    cells
}

fn sample_similarity_header_cell(
    projection: &SampleSimilarityHeaderProjection,
) -> ui::View<GuiMessage> {
    let mut header_parts = Vec::with_capacity(wavecrate_analysis::aspects::ASPECT_COUNT + 1);
    for aspect in &projection.aspects {
        let text = ui::text(aspect.label)
            .align_text(ui::TextAlign::Center)
            .key(identity::sample_similarity_header_aspect_key(aspect.label))
            .height(20.0)
            .width(SAMPLE_SIMILARITY_ASPECT_HEADER_WIDTH);
        header_parts.push(if aspect.enabled {
            text
        } else {
            text.muted_text()
        });
    }
    header_parts.push(
        ui::text(projection.score_label)
            .muted_text()
            .key(identity::SAMPLE_HEADER_SIMILARITY_LABEL_KEY)
            .height(20.0)
            .fill_width(),
    );
    ui::compact_details_cell(
        ui::row(header_parts).spacing(3.0).height(20.0).fill_width(),
        Some(SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH),
    )
    .key(identity::SAMPLE_HEADER_SIMILARITY_KEY)
}

fn sample_similarity_aspect_control(
    projection: SampleSimilarityAspectControlProjection,
) -> ui::View<GuiMessage> {
    ui::row([
        ui::color_marker(Some(similarity_aspect_color(projection.aspect)))
            .side(7)
            .inset(0)
            .view()
            .width(8.0)
            .height(20.0),
        ui::toggle(projection.label, projection.enabled)
            .subtle()
            .message(move |enabled| GuiMessage::SetSimilarityAspectEnabled {
                aspect: projection.aspect,
                enabled,
            })
            .id(identity::sample_similarity_aspect_toggle_id(
                projection.aspect,
            ))
            .size(34.0, 20.0),
        ui::slider(projection.weight)
            .compact()
            .subtle()
            .message(move |weight| GuiMessage::SetSimilarityAspectWeight {
                aspect: projection.aspect,
                weight,
            })
            .id(identity::sample_similarity_aspect_weight_id(
                projection.aspect,
            ))
            .size(62.0, 16.0),
    ])
    .spacing(3.0)
    .height(22.0)
}
