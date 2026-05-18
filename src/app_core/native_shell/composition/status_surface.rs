//! Generic status-bar surface projection for the native-shell compatibility layer.
//!
//! This module lets the compat shell compose one production footer slice from
//! public `radiant::layout`, `radiant::runtime`, and `radiant::widgets`
//! building blocks before the whole native window runtime migrates away from
//! the legacy `AppModel` path.

use super::style::SizingTokens;
use crate::{
    gui::types::{Point, Rect},
    layout::MainAlign,
    runtime::UiSurface,
};
use radiant::prelude as ui;
use radiant::prelude::IntoView;

const STATUS_ROOT_ID: u64 = 960;
const STATUS_ROW_ID: u64 = 961;
const STATUS_LEFT_SEGMENT_ID: u64 = 962;
const STATUS_CENTER_SEGMENT_ID: u64 = 963;
const STATUS_RIGHT_SEGMENT_ID: u64 = 964;
const STATUS_PROGRESS_SEGMENT_ID: u64 = 965;
const STATUS_LEFT_TEXT_ID: u64 = 970;
const STATUS_CENTER_TEXT_ID: u64 = 971;
const STATUS_RIGHT_TEXT_ID: u64 = 972;
const STATUS_PROGRESS_TEXT_ID: u64 = 973;
const STATUS_PROGRESS_TRACK_ID: u64 = 974;
const STATUS_LEFT_GAP_ID: u64 = 975;
const STATUS_RIGHT_GAP_ID: u64 = 976;
const STATUS_PROGRESS_GAP_ID: u64 = 977;
const STATUS_PROGRESS_ALIGN_ID: u64 = 978;
const STATUS_LEFT_RATIO: f32 = 0.30;
const STATUS_RIGHT_RATIO: f32 = 0.22;
const STATUS_PROGRESS_RATIO: f32 = 0.16;
const STATUS_PROGRESS_MIN_WIDTH: f32 = 84.0;
const STATUS_PROGRESS_MAX_WIDTH: f32 = 144.0;

/// User-facing content projected into the generic status-bar surface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusSurfaceContent {
    /// Left-aligned footer copy.
    pub left_label: String,
    /// Center footer copy. This becomes the inline progress label when active.
    pub center_label: String,
    /// Right-aligned footer copy.
    pub right_label: String,
    /// Counter shown in the compact progress slot.
    pub progress_counter: String,
}

/// Resolved layout rectangles for the generic status-bar surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatusSurfaceLayout {
    /// Left segment bounds.
    pub left_segment: Rect,
    /// Center segment bounds.
    pub center_segment: Rect,
    /// Right segment bounds.
    pub right_segment: Rect,
    /// Progress segment bounds.
    pub progress_segment: Rect,
    /// Left text widget bounds.
    pub left_text_rect: Rect,
    /// Center text widget bounds.
    pub center_text_rect: Rect,
    /// Right text widget bounds.
    pub right_text_rect: Rect,
    /// Progress counter widget bounds.
    pub progress_text_rect: Rect,
    /// Progress track canvas bounds.
    pub progress_track_rect: Rect,
}

/// Build a generic declarative status-bar surface for one footer snapshot.
pub(crate) fn build_status_surface(
    content: &StatusSurfaceContent,
    sizing: SizingTokens,
    viewport_width: f32,
) -> UiSurface<()> {
    let progress_width = progress_slot_width(viewport_width, sizing);
    let row = ui::row([
        segment_surface(
            STATUS_LEFT_SEGMENT_ID,
            STATUS_LEFT_TEXT_ID,
            &content.left_label,
            sizing,
        )
        .width_percent(STATUS_LEFT_RATIO)
        .fill_height(),
        ui::spacer()
            .id(STATUS_LEFT_GAP_ID)
            .width(sizing.status_segment_gap)
            .fill_height(),
        segment_surface(
            STATUS_CENTER_SEGMENT_ID,
            STATUS_CENTER_TEXT_ID,
            &content.center_label,
            sizing,
        )
        .fill(),
        ui::spacer()
            .id(STATUS_RIGHT_GAP_ID)
            .width(sizing.status_segment_gap)
            .fill_height(),
        segment_surface(
            STATUS_RIGHT_SEGMENT_ID,
            STATUS_RIGHT_TEXT_ID,
            &content.right_label,
            sizing,
        )
        .width_percent(STATUS_RIGHT_RATIO)
        .fill_height(),
        ui::spacer()
            .id(STATUS_PROGRESS_GAP_ID)
            .width(sizing.status_segment_gap)
            .fill_height(),
        progress_surface(content, sizing, progress_width)
            .width(progress_width)
            .fill_height(),
    ])
    .id(STATUS_ROW_ID)
    .spacing(0.0)
    .fill();
    UiSurface::new(
        ui::column([row])
            .id(STATUS_ROOT_ID)
            .padding_x(sizing.panel_inset.max(0.0))
            .fill()
            .into_node(),
    )
}

/// Resolve the generic status-bar surface layout inside one footer rect.
pub(crate) fn resolve_status_surface_layout(
    status_bar: Rect,
    sizing: SizingTokens,
    content: &StatusSurfaceContent,
) -> StatusSurfaceLayout {
    let surface = build_status_surface(content, sizing, status_bar.width());
    let output = surface.layout(status_bar);
    let empty = Rect::from_min_max(status_bar.min, status_bar.min);
    StatusSurfaceLayout {
        left_segment: output.rect_for_clamped(STATUS_LEFT_SEGMENT_ID, empty, status_bar),
        center_segment: output.rect_for_clamped(STATUS_CENTER_SEGMENT_ID, empty, status_bar),
        right_segment: output.rect_for_clamped(STATUS_RIGHT_SEGMENT_ID, empty, status_bar),
        progress_segment: output.rect_for_clamped(STATUS_PROGRESS_SEGMENT_ID, empty, status_bar),
        left_text_rect: output.rect_for_clamped(STATUS_LEFT_TEXT_ID, empty, status_bar),
        center_text_rect: output.rect_for_clamped(STATUS_CENTER_TEXT_ID, empty, status_bar),
        right_text_rect: output.rect_for_clamped(STATUS_RIGHT_TEXT_ID, empty, status_bar),
        progress_text_rect: output.rect_for_clamped(STATUS_PROGRESS_TEXT_ID, empty, status_bar),
        progress_track_rect: output.rect_for_clamped(STATUS_PROGRESS_TRACK_ID, empty, status_bar),
    }
}

fn segment_surface(
    segment_id: u64,
    text_id: u64,
    label: &str,
    sizing: SizingTokens,
) -> ui::View<()> {
    let text_height = sizing.font_status.max(1.0);
    ui::column([ui::text(label)
        .id(text_id)
        .size(1.0, text_height)
        .baseline((text_height * 0.75).max(0.0))
        .fill_width()
        .height(text_height)])
    .id(segment_id)
    .padding_x((sizing.text_inset_x + sizing.header_label_gutter).max(0.0))
    .padding_y(sizing.text_inset_y.max(0.0))
    .align_main(MainAlign::Center)
    .fill()
}

fn progress_surface(
    content: &StatusSurfaceContent,
    sizing: SizingTokens,
    progress_width: f32,
) -> ui::View<()> {
    let track_height = (sizing.border_width * 2.0).max(4.0);
    let text_height = sizing.font_status.max(1.0);
    ui::column([
        ui::spacer().id(STATUS_PROGRESS_ALIGN_ID).fill(),
        ui::text(&content.progress_counter)
            .id(STATUS_PROGRESS_TEXT_ID)
            .size(progress_width, text_height)
            .baseline((text_height * 0.75).max(0.0))
            .fill_width()
            .height(text_height),
        ui::canvas()
            .id(STATUS_PROGRESS_TRACK_ID)
            .size(progress_width, track_height)
            .fill_width()
            .height(track_height),
    ])
    .id(STATUS_PROGRESS_SEGMENT_ID)
    .spacing(0.0)
    .padding_x((sizing.text_inset_x + sizing.header_label_gutter).max(0.0))
    .padding_y(sizing.text_inset_y.max(0.0))
    .fill()
}

fn progress_slot_width(viewport_width: f32, sizing: SizingTokens) -> f32 {
    let inner_width = (viewport_width - (sizing.panel_inset.max(0.0) * 2.0)).max(0.0);
    (inner_width * STATUS_PROGRESS_RATIO)
        .clamp(STATUS_PROGRESS_MIN_WIDTH, STATUS_PROGRESS_MAX_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::native_shell::composition::style::StyleTokens;

    fn assert_widget_node(surface: &UiSurface<()>, id: u64) {
        assert_eq!(
            surface
                .find_widget(id)
                .expect("widget node should exist")
                .id(),
            id
        );
    }

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn status_surface_projects_radiant_primitives() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let surface = build_status_surface(
            &StatusSurfaceContent {
                left_label: String::from("Transport: running"),
                center_label: String::from("rows: 20 | selected: 2"),
                right_label: String::from("col: 2/3"),
                progress_counter: String::from("4/9"),
            },
            style.sizing,
            1280.0,
        );
        assert_widget_node(&surface, STATUS_LEFT_TEXT_ID);
        assert_widget_node(&surface, STATUS_PROGRESS_TRACK_ID);
    }

    #[test]
    fn status_surface_layout_keeps_segments_ordered() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let bar = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 20.0));
        let layout =
            resolve_status_surface_layout(bar, style.sizing, &StatusSurfaceContent::default());
        assert_inside(bar, layout.left_segment);
        assert_inside(bar, layout.center_segment);
        assert_inside(bar, layout.right_segment);
        assert_inside(bar, layout.progress_segment);
        assert!(layout.left_segment.max.x <= layout.center_segment.min.x);
        assert!(layout.center_segment.max.x <= layout.right_segment.min.x);
        assert!(layout.right_segment.max.x <= layout.progress_segment.min.x);
    }

    #[test]
    fn status_surface_text_and_progress_widgets_stay_inside_footer() {
        let style = StyleTokens::for_viewport_width(820.0);
        let bar = Rect::from_min_max(Point::new(10.0, 5.0), Point::new(360.0, 24.0));
        let layout =
            resolve_status_surface_layout(bar, style.sizing, &StatusSurfaceContent::default());
        assert_inside(layout.left_segment, layout.left_text_rect);
        assert_inside(layout.center_segment, layout.center_text_rect);
        assert_inside(layout.right_segment, layout.right_text_rect);
        assert_inside(layout.progress_segment, layout.progress_text_rect);
        assert_inside(layout.progress_segment, layout.progress_track_rect);
    }
}
