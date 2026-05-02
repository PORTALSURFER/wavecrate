//! Slotized browser map-header text layout helpers.

use super::super::style::SizingTokens;
use crate::gui::text_layout::{TextLineInsets, centered_text_line};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const MAP_HEADER_ROOT_ID: u64 = 1300;
const MAP_HEADER_ROW_ID: u64 = 1301;
const MAP_HEADER_LEFT_ID: u64 = 1302;
const MAP_HEADER_RIGHT_ID: u64 = 1303;
const MAP_HEADER_TEXT_LINE_ID: u64 = 1312;
const RIGHT_LABEL_RATIO: f32 = 0.42;
const RIGHT_LABEL_MIN_WIDTH: f32 = 36.0;

/// Slot-resolved map-header left and right label bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserMapHeaderTextLayout {
    pub left_label: Rect,
    pub right_label: Rect,
}

/// Compute browser map-header label bounds through strict slot layout.
pub(crate) fn compute_browser_map_header_text_layout(
    header_rect: Rect,
    sizing: SizingTokens,
) -> BrowserMapHeaderTextLayout {
    let empty = empty_rect(header_rect);
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return BrowserMapHeaderTextLayout {
            left_label: empty,
            right_label: empty,
        };
    }
    let right_width = (header_rect.width() * RIGHT_LABEL_RATIO)
        .max(RIGHT_LABEL_MIN_WIDTH)
        .min(header_rect.width());
    let tree = LayoutNode::container(
        MAP_HEADER_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: 0.0,
                bottom: 0.0,
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                MAP_HEADER_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(MAP_HEADER_LEFT_ID, Vector2::new(1.0, 1.0)),
                    },
                    fixed_width_child(MAP_HEADER_RIGHT_ID, right_width),
                ],
            ),
        }],
    );
    let output = layout_tree(&tree, header_rect);
    let left_bounds = clamp_rect_to_bounds(
        rect_for(&output.rects, MAP_HEADER_LEFT_ID, empty),
        header_rect,
    );
    let right_bounds = clamp_rect_to_bounds(
        rect_for(&output.rects, MAP_HEADER_RIGHT_ID, empty),
        header_rect,
    );
    BrowserMapHeaderTextLayout {
        left_label: compute_map_header_text_line(left_bounds, sizing, sizing.font_meta),
        right_label: compute_map_header_text_line(right_bounds, sizing, sizing.font_meta),
    }
}

fn fixed_width_child(node_id: u64, width: f32) -> SlotChild {
    let width = width.max(0.0);
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width, width, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

fn compute_map_header_text_line(rect: Rect, sizing: SizingTokens, font_size: f32) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        rect,
        font_size,
        TextLineInsets {
            left: 0.0,
            right: 0.0,
            top: sizing.text_inset_y.max(0.0),
            bottom: sizing.text_inset_y.max(0.0),
        },
        0.0,
        MAP_HEADER_TEXT_LINE_ID,
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
    fn map_header_labels_stay_inside_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(220.0, 278.0), Point::new(1260.0, 296.0));
        let layout = compute_browser_map_header_text_layout(header, style.sizing);
        assert_inside(header, layout.left_label);
        assert_inside(header, layout.right_label);
    }

    #[test]
    fn map_header_right_label_stays_in_right_partition() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(960.0, 26.0));
        let layout = compute_browser_map_header_text_layout(header, style.sizing);
        assert!(layout.right_label.min.x >= header.min.x + (header.width() * 0.5));
    }

    #[test]
    fn map_header_labels_collapse_for_empty_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        let layout = compute_browser_map_header_text_layout(header, style.sizing);
        let empty = Rect::from_min_max(header.min, header.min);
        assert_eq!(layout.left_label, empty);
        assert_eq!(layout.right_label, empty);
    }
}
