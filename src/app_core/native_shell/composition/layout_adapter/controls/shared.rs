#![allow(dead_code)]
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Rect, Vector2};

pub(super) fn center_square_rect(rect: Rect, side: f32) -> Rect {
    rect.centered_square(side)
}

#[cfg(test)]
pub(super) fn clamp_rect_right_edge(rect: Rect, bounds: Rect, right_edge: f32) -> Rect {
    let clamped = clamp_rect_to_bounds(rect, bounds);
    let max_x = clamped.max.x.min(right_edge.max(bounds.min.x));
    if max_x < clamped.min.x {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(
        clamped.min,
        crate::gui::types::Point::new(max_x, clamped.max.y),
    )
}

pub(super) fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    rect.clamp_to(bounds)
}

pub(super) fn empty_rect(bounds: Rect) -> Rect {
    bounds.empty_at_min()
}

pub(super) fn layout_left_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
    }
    let mut children = Vec::with_capacity(widths.len());
    for (index, width) in widths.iter().enumerate() {
        children.push(fixed_width_child(
            first_button_id + index as u64,
            *width,
            if index == 0 { 0.0 } else { gap },
        ));
    }
    let tree = LayoutNode::container(
        row_id,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(&tree, bounds);
    widths
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let id = first_button_id + index as u64;
            let rect = rect_for(&output.rects, id, empty_rect(bounds));
            clamp_rect_to_bounds(rect, bounds)
        })
        .collect()
}

pub(super) fn layout_right_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    spacer_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
    }
    let mut children = Vec::with_capacity(widths.len() + 1);
    children.push(SlotChild {
        slot: SlotParams::fill(),
        child: LayoutNode::widget(spacer_id, Vector2::new(1.0, 1.0)),
    });
    for (index, width) in widths.iter().enumerate() {
        children.push(fixed_width_child(
            first_button_id + index as u64,
            *width,
            if index == 0 { 0.0 } else { gap },
        ));
    }
    let tree = LayoutNode::container(
        row_id,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(&tree, bounds);
    widths
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let id = first_button_id + index as u64;
            let rect = rect_for(&output.rects, id, empty_rect(bounds));
            clamp_rect_to_bounds(rect, bounds)
        })
        .collect()
}

pub(super) fn rect_for(
    rects: &std::collections::BTreeMap<u64, Rect>,
    id: u64,
    fallback: Rect,
) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

pub(super) fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    if available_width <= 0.0 || widths.is_empty() {
        return Vec::new();
    }
    let mut used = 0.0;
    let mut reversed = Vec::new();
    for (index, width) in widths.iter().rev().enumerate() {
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate >= available_width {
            break;
        }
        reversed.push(*width);
        used = candidate;
    }
    reversed.reverse();
    reversed
}

fn fixed_width_child(node_id: u64, width: f32, left_margin: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width.max(0.0), width.max(0.0), 0.0, f32::INFINITY),
            margin: Insets {
                left: left_margin.max(0.0),
                ..Insets::default()
            },
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}
