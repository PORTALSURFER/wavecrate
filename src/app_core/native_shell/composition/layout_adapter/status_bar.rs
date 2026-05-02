//! Slotized helpers for status-bar segment and text-line geometry.

use super::super::status_surface::{StatusSurfaceContent, resolve_status_surface_layout};
use super::super::style::SizingTokens;
use crate::gui::text_layout::{TextLineInsets, centered_text_line};
use crate::gui::types::Rect;

const STATUS_TEXT_LINE_ID: u64 = 922;

/// Slot-resolved left/center/right status-bar segment geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatusBarSegments {
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
    pub progress: Rect,
}

/// Compute left/center/right status-bar segments through slotized layout.
pub(crate) fn compute_status_bar_segments(
    status_bar: Rect,
    sizing: SizingTokens,
) -> StatusBarSegments {
    let layout =
        resolve_status_surface_layout(status_bar, sizing, &StatusSurfaceContent::default());
    StatusBarSegments {
        left: layout.left_segment,
        center: layout.center_segment,
        right: layout.right_segment,
        progress: layout.progress_segment,
    }
}

/// Compute a status text line bounds rect inside a status segment.
pub(crate) fn compute_status_text_line_rect(
    segment: Rect,
    sizing: SizingTokens,
    font_size: f32,
) -> Rect {
    let empty = empty_rect(segment);
    if segment.width() <= 0.0 || segment.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        segment,
        font_size,
        TextLineInsets {
            left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
            right: sizing.text_inset_x.max(0.0),
            top: sizing.text_inset_y.max(0.0),
            bottom: sizing.text_inset_y.max(0.0),
        },
        0.0,
        STATUS_TEXT_LINE_ID,
    )
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;
    use crate::gui::types::Point;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn status_segments_preserve_order_and_spacing() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let bar = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 20.0));
        let segments = compute_status_bar_segments(bar, style.sizing);
        assert_inside(bar, segments.left);
        assert_inside(bar, segments.center);
        assert_inside(bar, segments.right);
        assert_inside(bar, segments.progress);
        assert!(segments.left.max.x <= segments.center.min.x);
        assert!(segments.center.max.x <= segments.right.min.x);
        assert!(segments.right.max.x <= segments.progress.min.x);
    }

    #[test]
    fn status_segments_clamp_when_bar_is_narrow() {
        let style = StyleTokens::for_viewport_width(820.0);
        let bar = Rect::from_min_max(Point::new(10.0, 5.0), Point::new(74.0, 20.0));
        let segments = compute_status_bar_segments(bar, style.sizing);
        assert_inside(bar, segments.left);
        assert_inside(bar, segments.center);
        assert_inside(bar, segments.right);
        assert_inside(bar, segments.progress);
    }

    #[test]
    fn status_text_rect_stays_within_segment() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let segment = Rect::from_min_max(Point::new(20.0, 4.0), Point::new(380.0, 20.0));
        let text_rect =
            compute_status_text_line_rect(segment, style.sizing, style.sizing.font_status);
        assert_inside(segment, text_rect);
        assert!(text_rect.width() > 0.0);
        assert!(text_rect.height() > 0.0);
    }

    #[test]
    fn status_text_rect_collapses_for_invalid_segment() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let segment = Rect::from_min_max(Point::new(20.0, 4.0), Point::new(20.0, 4.0));
        let text_rect =
            compute_status_text_line_rect(segment, style.sizing, style.sizing.font_status);
        assert_eq!(text_rect, Rect::from_min_max(segment.min, segment.min));
    }
}
