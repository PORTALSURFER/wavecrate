use radiant::prelude as ui;
use radiant::runtime::{
    PaintBrush, PaintFillPath, PaintLinearGradient, PaintPath, PaintPathCommand, PaintPrimitive,
    TransientOverlayContext,
};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::CollectionsSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING, COLLECTIONS_PANEL_PADDING,
    SIDEBAR_PANEL_HEADER_HEIGHT,
};
use crate::native_app::ui::ids as widget_ids;

mod identity;
mod rows;
#[cfg(test)]
mod tests;

use super::edge_aligned_resize_panel;
use rows::collection_row;

/// Stable layout node id for collection-panel resize regression coverage.
const COLLECTIONS_SECTION_NODE_ID: u64 = widget_ids::COLLECTIONS_SECTION_NODE_ID;
/// Stable layout node id for the collection rows scroll viewport.
const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = widget_ids::COLLECTIONS_LIST_SCROLL_NODE_ID;
const COLLECTIONS_RESIZE_HEADER_ID: u64 = widget_ids::COLLECTIONS_RESIZE_HEADER_ID;
const COLLECTIONS_OVERFLOW_FADE_ID: u64 = 0x636f_6c6c_5f66_6164;
const COLLECTIONS_OVERFLOW_FADE_HEIGHT: f32 = 48.0;
const COLLECTIONS_OVERFLOW_FADE_RAMP: f32 = 12.0;
const COLLECTIONS_OVERFLOW_FADE_MAX_ALPHA: u8 = u8::MAX;

pub(super) fn collections_section(model: &CollectionsSectionViewModel) -> ui::View<GuiMessage> {
    let rows = model.rows.iter().map(collection_row).collect::<Vec<_>>();
    edge_aligned_resize_panel(
        "collections-resize-header",
        COLLECTIONS_RESIZE_HEADER_ID,
        SIDEBAR_PANEL_HEADER_HEIGHT,
        ui::scroll(
            ui::column(rows)
                .spacing(COLLECTION_ROW_SPACING)
                .fill_width()
                .height(model.list_height),
        )
        .id(COLLECTIONS_LIST_SCROLL_NODE_ID)
        .fill_width()
        .fill_height(),
        model.panel_height,
        COLLECTIONS_PANEL_PADDING,
        COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
        |message| GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message)),
    )
    .id(COLLECTIONS_SECTION_NODE_ID)
}

/// Returns the opacity for the passive bottom-edge affordance.
///
/// The fade becomes visible only after the resizable panel begins clipping its
/// useful content, then reaches its intended strength over roughly one row.
pub(in crate::native_app) fn collection_overflow_fade_alpha(
    panel_height: f32,
    full_height: f32,
) -> u8 {
    let clipped_height = (full_height - panel_height).max(0.0);
    let strength = (clipped_height / COLLECTIONS_OVERFLOW_FADE_RAMP).clamp(0.0, 1.0);
    (f32::from(COLLECTIONS_OVERFLOW_FADE_MAX_ALPHA) * strength).round() as u8
}

/// Paints a non-interactive gradient that softens a clipped collection list.
pub(in crate::native_app) fn paint_collection_overflow_fade(
    context: TransientOverlayContext<'_>,
    opacity: u8,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let Some(bounds) = context
        .plan
        .primitives
        .iter()
        .filter_map(PaintPrimitive::clip_start)
        .find(|clip| clip.node_id == COLLECTIONS_LIST_SCROLL_NODE_ID)
        .map(|clip| clip.rect)
    else {
        return;
    };
    let Some((fade_bounds, gradient)) =
        collection_overflow_fade_gradient(bounds, opacity, context.plan.clear_color)
    else {
        return;
    };
    primitives.push(PaintPrimitive::FillPath(PaintFillPath::new(
        COLLECTIONS_OVERFLOW_FADE_ID,
        rectangle_path(fade_bounds),
        PaintBrush::linear_gradient(gradient),
    )));
}

fn collection_overflow_fade_gradient(
    bounds: ui::Rect,
    opacity: u8,
    background: ui::Rgba8,
) -> Option<(ui::Rect, PaintLinearGradient)> {
    if opacity == 0 || !bounds.has_finite_positive_area() {
        return None;
    }
    let height = COLLECTIONS_OVERFLOW_FADE_HEIGHT.min(bounds.height());
    if height <= 0.0 {
        return None;
    }
    let fade_bounds = ui::Rect::from_min_max(
        ui::Point::new(bounds.min.x, bounds.max.y - height),
        bounds.max,
    );
    // A slightly deeper ending tone keeps the fade readable in sparse rows,
    // while still resolving to the sidebar's neutral background rather than a
    // competing surface color.
    let bottom = ui::Rgba8::new(
        background.r.saturating_sub(8),
        background.g.saturating_sub(8),
        background.b.saturating_sub(8),
        opacity,
    );
    Some((
        fade_bounds,
        PaintLinearGradient::vertical(fade_bounds, bottom.with_alpha(0), bottom),
    ))
}

fn rectangle_path(rect: ui::Rect) -> PaintPath {
    PaintPath::from([
        PaintPathCommand::MoveTo(rect.min),
        PaintPathCommand::LineTo(ui::Point::new(rect.max.x, rect.min.y)),
        PaintPathCommand::LineTo(rect.max),
        PaintPathCommand::LineTo(ui::Point::new(rect.min.x, rect.max.y)),
        PaintPathCommand::Close,
    ])
}
