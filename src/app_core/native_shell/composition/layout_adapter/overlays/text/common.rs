use crate::gui::text_layout::{TextLineInsets, centered_text_line, top_text_line};
use super::super::shared;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Rect, Vector2};

pub(super) fn centered_line_in_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    node_id: u64,
) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        shared::clamp_rect_to_bounds(rect, rect),
        font_size,
        TextLineInsets::symmetric(sizing.text_inset_x.max(0.0), sizing.text_inset_y.max(0.0)),
        0.0,
        node_id,
    )
}

pub(super) fn fixed_height_child(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height.max(0.0), height.max(0.0)),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: true,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

pub(super) fn top_line_in_bounds(bounds: Rect, font_size: f32, node_id: u64) -> Rect {
    let empty = shared::empty_rect(bounds);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    top_text_line(bounds, font_size, TextLineInsets::horizontal(0.0), node_id)
}

pub(super) fn top_line_in_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    node_id: u64,
) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    top_text_line(
        shared::clamp_rect_to_bounds(rect, rect),
        font_size,
        TextLineInsets::horizontal(sizing.text_inset_x.max(0.0)),
        node_id,
    )
}

pub(super) fn column_tree(root_id: u64, children: Vec<SlotChild>) -> LayoutNode {
    LayoutNode::container(
        root_id,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    )
}
