//! Slotized waveform header text-row layout helpers.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const WAVEFORM_HEADER_ROOT_ID: u64 = 1100;
const WAVEFORM_HEADER_COLUMN_ID: u64 = 1101;
const WAVEFORM_HEADER_TITLE_ID: u64 = 1102;
const WAVEFORM_HEADER_META_ID: u64 = 1103;
const WAVEFORM_HEADER_FILL_ID: u64 = 1104;

/// Slot-resolved waveform header title + metadata text rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformHeaderTextLayout {
    pub title_row: Rect,
    pub metadata_row: Rect,
}

/// Compute waveform header title and metadata text rows through slotized layout.
pub(crate) fn compute_waveform_header_text_layout(
    header_rect: Rect,
    sizing: SizingTokens,
) -> WaveformHeaderTextLayout {
    let empty = empty_rect(header_rect);
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return WaveformHeaderTextLayout {
            title_row: empty,
            metadata_row: empty,
        };
    }
    let tree = waveform_header_tree(sizing);
    let output = layout_tree(&tree, header_rect);
    let title_row = clamp_rect_to_bounds(
        rect_for(&output.rects, WAVEFORM_HEADER_TITLE_ID, empty),
        header_rect,
    );
    let metadata_row = clamp_rect_to_bounds(
        rect_for(&output.rects, WAVEFORM_HEADER_META_ID, empty),
        header_rect,
    );
    WaveformHeaderTextLayout {
        title_row,
        metadata_row,
    }
}

fn waveform_header_tree(sizing: SizingTokens) -> LayoutNode {
    LayoutNode::container(
        WAVEFORM_HEADER_ROOT_ID,
        root_policy(sizing),
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: waveform_header_column(sizing),
        }],
    )
}

fn root_policy(sizing: SizingTokens) -> ContainerPolicy {
    ContainerPolicy {
        kind: ContainerKind::PaddingBox,
        padding: Insets {
            left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
            right: sizing.text_inset_x.max(0.0),
            top: sizing.text_inset_y.max(0.0),
            bottom: sizing.text_inset_y.max(0.0),
        },
        align_cross: CrossAlign::Stretch,
        overflow: OverflowPolicy::Clip,
        ..ContainerPolicy::default()
    }
}

fn waveform_header_column(sizing: SizingTokens) -> LayoutNode {
    LayoutNode::container(
        WAVEFORM_HEADER_COLUMN_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: sizing.text_row_gap.max(0.0),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            fixed_height_row(WAVEFORM_HEADER_TITLE_ID, sizing.font_header.max(1.0)),
            fixed_height_row(WAVEFORM_HEADER_META_ID, sizing.font_meta.max(1.0)),
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(WAVEFORM_HEADER_FILL_ID, Vector2::new(1.0, 1.0)),
            },
        ],
    )
}

fn fixed_height_row(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height, height),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
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
    fn waveform_header_rows_stay_inside_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(220.0, 32.0), Point::new(1260.0, 64.0));
        let rows = compute_waveform_header_text_layout(header, style.sizing);
        assert_inside(header, rows.title_row);
        assert_inside(header, rows.metadata_row);
    }

    #[test]
    fn waveform_header_rows_preserve_vertical_order() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(960.0, 40.0));
        let rows = compute_waveform_header_text_layout(header, style.sizing);
        assert!(rows.title_row.min.y <= rows.metadata_row.min.y);
        assert!(rows.title_row.max.y <= rows.metadata_row.max.y);
    }

    #[test]
    fn waveform_header_rows_collapse_for_empty_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        let rows = compute_waveform_header_text_layout(header, style.sizing);
        let empty = Rect::from_min_max(header.min, header.min);
        assert_eq!(rows.title_row, empty);
        assert_eq!(rows.metadata_row, empty);
    }
}
