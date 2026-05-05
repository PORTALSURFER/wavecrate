//! Slotized text-line geometry helpers for sidebar header/footer chrome copy.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const SIDEBAR_HEADER_ROOT_ID: u64 = 1730;
const SIDEBAR_HEADER_TITLE_ID: u64 = 1731;
const SIDEBAR_HEADER_QUERY_ID: u64 = 1732;
const SIDEBAR_FOOTER_ROOT_ID: u64 = 1740;
const SIDEBAR_FOOTER_PRIMARY_ID: u64 = 1741;
const SIDEBAR_FOOTER_SECONDARY_ID: u64 = 1742;

#[derive(Clone, Copy)]
struct TwoRowSpec {
    root_id: u64,
    first_id: u64,
    second_id: u64,
    first_height: f32,
    second_height: f32,
}

/// Slot-resolved row bounds for sidebar sources header copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarHeaderTextLayout {
    pub title_row: Rect,
    pub query_row: Rect,
}

/// Slot-resolved row bounds for sidebar footer summary copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarFooterTextLayout {
    pub primary_row: Rect,
    pub secondary_row: Rect,
}

/// Compute sidebar-header title/query line bounds via strict slotized layout.
pub(crate) fn compute_sidebar_header_text_layout(
    header_rect: Rect,
    sizing: SizingTokens,
) -> SidebarHeaderTextLayout {
    let bounds = sidebar_text_bounds(header_rect, sizing);
    let rows = compute_two_rows(
        bounds,
        sizing.text_inset_y.max(0.0),
        sizing.text_row_gap.max(0.0),
        TwoRowSpec {
            root_id: SIDEBAR_HEADER_ROOT_ID,
            first_id: SIDEBAR_HEADER_TITLE_ID,
            second_id: SIDEBAR_HEADER_QUERY_ID,
            first_height: sizing.font_header.max(1.0),
            second_height: sizing.font_meta.max(1.0),
        },
    );
    SidebarHeaderTextLayout {
        title_row: rows[0],
        query_row: rows[1],
    }
}

/// Compute sidebar-footer summary line bounds via strict slotized layout.
pub(crate) fn compute_sidebar_footer_text_layout(
    footer_rect: Rect,
    sizing: SizingTokens,
) -> SidebarFooterTextLayout {
    let bounds = sidebar_text_bounds(footer_rect, sizing);
    let rows = compute_two_rows(
        bounds,
        sizing.text_inset_y.max(0.0),
        sizing.text_row_gap.max(0.0),
        TwoRowSpec {
            root_id: SIDEBAR_FOOTER_ROOT_ID,
            first_id: SIDEBAR_FOOTER_PRIMARY_ID,
            second_id: SIDEBAR_FOOTER_SECONDARY_ID,
            first_height: sizing.font_meta.max(1.0),
            second_height: sizing.font_meta.max(1.0),
        },
    );
    SidebarFooterTextLayout {
        primary_row: rows[0],
        secondary_row: rows[1],
    }
}

fn compute_two_rows(bounds: Rect, inset_y: f32, row_gap: f32, spec: TwoRowSpec) -> [Rect; 2] {
    let empty = empty_rect(bounds);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return [empty, empty];
    }
    let tree = two_row_tree(inset_y, row_gap, spec);
    let output = layout_tree(&tree, bounds);
    [
        clamp_rect_to_bounds(rect_for(&output.rects, spec.first_id, empty), bounds),
        clamp_rect_to_bounds(rect_for(&output.rects, spec.second_id, empty), bounds),
    ]
}

fn two_row_tree(inset_y: f32, row_gap: f32, spec: TwoRowSpec) -> LayoutNode {
    LayoutNode::container(
        spec.root_id,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                top: inset_y,
                bottom: inset_y,
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                spec.root_id + 10,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: row_gap,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    line_slot_child(spec.first_id, spec.first_height),
                    line_slot_child(spec.second_id, spec.second_height),
                ],
            ),
        }],
    )
}

fn line_slot_child(node_id: u64, line_height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(line_height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, line_height, line_height),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, line_height)),
    }
}

fn sidebar_text_bounds(rect: Rect, sizing: SizingTokens) -> Rect {
    let left = rect.min.x + sizing.text_inset_x + sizing.header_label_gutter;
    let width = (rect.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    let right = (left + width).min(rect.max.x);
    let min_x = left.min(rect.max.x);
    let max_x = right.max(min_x);
    Rect::from_min_max(
        Point::new(min_x, rect.min.y),
        Point::new(max_x, rect.max.y.max(rect.min.y)),
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
    fn sidebar_header_text_rows_stay_inside_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rect = Rect::from_min_max(Point::new(8.0, 80.0), Point::new(208.0, 134.0));
        let layout = compute_sidebar_header_text_layout(rect, style.sizing);
        assert_inside(rect, layout.title_row);
        assert_inside(rect, layout.query_row);
        assert!(layout.query_row.min.y >= layout.title_row.max.y);
    }

    #[test]
    fn sidebar_footer_text_rows_stay_inside_footer() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rect = Rect::from_min_max(Point::new(8.0, 500.0), Point::new(208.0, 552.0));
        let layout = compute_sidebar_footer_text_layout(rect, style.sizing);
        assert_inside(rect, layout.primary_row);
        assert_inside(rect, layout.secondary_row);
        assert!(layout.secondary_row.min.y >= layout.primary_row.max.y);
    }

    #[test]
    fn sidebar_header_text_respects_left_inset() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rect = Rect::from_min_max(Point::new(8.0, 80.0), Point::new(208.0, 134.0));
        let layout = compute_sidebar_header_text_layout(rect, style.sizing);
        let expected_left =
            rect.min.x + style.sizing.text_inset_x + style.sizing.header_label_gutter;
        assert!(layout.title_row.min.x >= expected_left);
        assert!(layout.query_row.min.x >= expected_left);
    }
}
