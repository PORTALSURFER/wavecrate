//! Slotized sidebar header/rows/footer band resolution.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutDebugOptions,
    LayoutEngine, LayoutNode, LayoutState, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain,
    SlotChild, SlotParams,
};
use crate::gui::types::{Point, Rect, Vector2};

pub(crate) const SIDEBAR_BANDS_ROOT_ID: u64 = 630;
pub(crate) const SIDEBAR_HEADER_ID: u64 = 632;
pub(crate) const SIDEBAR_ROWS_ID: u64 = 633;
pub(crate) const SIDEBAR_FOOTER_ID: u64 = 634;

/// Slot-resolved sidebar band rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarBandSections {
    pub sidebar_header: Rect,
    pub sidebar_rows: Rect,
    pub sidebar_footer: Rect,
}

/// Compute sidebar band sections with a caller-provided persistent layout engine.
pub(crate) fn compute_sidebar_band_sections_with_layout_engine(
    sidebar: Rect,
    sizing: SizingTokens,
    engine: &mut LayoutEngine,
    state: &LayoutState,
) -> SidebarBandSections {
    let empty = empty_rect(sidebar);
    if sidebar.width() <= 0.0 || sidebar.height() <= 0.0 {
        return SidebarBandSections {
            sidebar_header: empty,
            sidebar_rows: empty,
            sidebar_footer: empty,
        };
    }
    let section_tree = build_sidebar_bands_tree(sidebar, sizing);
    let output =
        engine.layout_with_state(&section_tree, sidebar, state, LayoutDebugOptions::default());
    let sidebar_rows = rect_for(&output.rects, SIDEBAR_ROWS_ID, empty);
    SidebarBandSections {
        sidebar_header: clamp_rect_to_bounds(
            rect_for(&output.rects, SIDEBAR_HEADER_ID, empty),
            sidebar,
        ),
        sidebar_rows: clamp_rect_to_bounds(sidebar_rows, sidebar),
        sidebar_footer: clamp_rect_to_bounds(
            rect_for(&output.rects, SIDEBAR_FOOTER_ID, empty),
            sidebar,
        ),
    }
}

/// Build sidebar header/rows/footer tree for persistent runtime caching.
pub(crate) fn build_sidebar_bands_tree(sidebar: Rect, sizing: SizingTokens) -> LayoutNode {
    let header_height = sizing
        .source_header_block_height
        .min(sidebar.height().max(0.0));
    let footer_height = (sizing.source_bottom_padding
        + sizing.sidebar_action_button_height
        + (sizing.text_inset_y * 2.0))
        .max(sizing.sidebar_action_button_height + 1.0)
        .min(sidebar.height().max(0.0));
    LayoutNode::container(
        SIDEBAR_BANDS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            fixed_height_child(SIDEBAR_HEADER_ID, header_height, 0.0),
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(SIDEBAR_ROWS_ID, Vector2::new(1.0, 1.0)),
            },
            fixed_height_child(SIDEBAR_FOOTER_ID, footer_height, 0.0),
        ],
    )
}

fn fixed_height_child(node_id: u64, height: f32, bottom_margin: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, 0.0, height.max(0.0)),
            margin: Insets {
                bottom: bottom_margin.max(0.0),
                ..Insets::default()
            },
            align_cross_override: None,
            allow_fixed_compress: true,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}
