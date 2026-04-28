//! Slotized helpers for browser-map canvas and point anchor geometry.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, OverflowPolicy, SlotChild,
    SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const MAP_CANVAS_ROOT_ID: u64 = 1760;
const MAP_CANVAS_FILL_ID: u64 = 1761;

/// Compute the browser-map canvas rect inside browser-row bounds.
pub(crate) fn compute_browser_map_canvas_rect(browser_rows: Rect, sizing: SizingTokens) -> Rect {
    let empty = empty_rect(browser_rows);
    if browser_rows.width() <= 0.0 || browser_rows.height() <= 0.0 {
        return empty;
    }
    let inset = (sizing.text_inset_x * 0.5).max(2.0);
    let tree = LayoutNode::container(
        MAP_CANVAS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: inset,
                right: inset,
                top: inset,
                bottom: inset,
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(MAP_CANVAS_FILL_ID, Vector2::new(1.0, 1.0)),
        }],
    );
    let output = layout_tree(&tree, browser_rows);
    clamp_rect_to_bounds(
        rect_for(&output.rects, MAP_CANVAS_FILL_ID, empty),
        browser_rows,
    )
}

/// Compute a point center within the map canvas from normalized milli coords.
pub(crate) fn compute_browser_map_point_center(canvas: Rect, x_milli: u16, y_milli: u16) -> Point {
    let x_ratio = f32::from(x_milli.min(1000)) / 1000.0;
    let y_ratio = f32::from(y_milli.min(1000)) / 1000.0;
    Point::new(
        canvas.min.x + (canvas.width().max(0.0) * x_ratio),
        canvas.min.y + (canvas.height().max(0.0) * y_ratio),
    )
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn map_canvas_rect_stays_inside_browser_rows() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rows = Rect::from_min_max(Point::new(200.0, 320.0), Point::new(1280.0, 860.0));
        let canvas = compute_browser_map_canvas_rect(rows, style.sizing);
        assert_inside(rows, canvas);
        assert!(canvas.min.x > rows.min.x);
        assert!(canvas.min.y > rows.min.y);
    }

    #[test]
    fn map_canvas_rect_collapses_for_empty_rows() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rows = Rect::from_min_max(Point::new(200.0, 320.0), Point::new(200.0, 320.0));
        let canvas = compute_browser_map_canvas_rect(rows, style.sizing);
        assert_eq!(canvas, rows);
    }

    #[test]
    fn map_point_center_clamps_normalized_milli() {
        let canvas = Rect::from_min_max(Point::new(100.0, 200.0), Point::new(300.0, 500.0));
        let center = compute_browser_map_point_center(canvas, 1400, 1300);
        assert_eq!(center, canvas.max);
    }
}
