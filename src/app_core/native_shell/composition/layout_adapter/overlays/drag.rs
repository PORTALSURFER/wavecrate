//! Slotized drag-overlay geometry.

use super::shared;
use crate::gui::layout_core::{
    ContainerKind, ContainerPolicy, CrossAlign, LayoutNode, MainAlign, OverflowPolicy, layout_tree,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect};

const DRAG_OVERLAY_ALIGN_ROOT_ID: u64 = 960;
const DRAG_OVERLAY_ID: u64 = 961;

/// Compute drag overlay banner rect between content and status bars.
pub(super) fn compute_drag_overlay_rect(
    content: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
) -> Rect {
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return shared::empty_rect(content);
    }
    let width = (content.width() * 0.72).clamp(260.0, 520.0);
    let height = sizing.drag_overlay_height.max(0.0);
    let min_y = content.min.y + sizing.overlay_padding.max(0.0);
    let max_y = content.max.y.min(status_bar.min.y - 1.0);
    if max_y <= min_y || width <= 0.0 || height <= 0.0 {
        return shared::empty_rect(content);
    }
    let align_bounds = Rect::from_min_max(
        Point::new(content.min.x, min_y),
        Point::new(
            content.max.x,
            (max_y - (sizing.panel_gap - 1.0).max(0.0)).max(min_y),
        ),
    );
    if align_bounds.width() <= 0.0 || align_bounds.height() <= 0.0 {
        return shared::empty_rect(content);
    }
    let tree = LayoutNode::container(
        DRAG_OVERLAY_ALIGN_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::End,
            align_cross: CrossAlign::Center,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![shared::fixed_child(
            DRAG_OVERLAY_ID,
            width,
            height,
            CrossAlign::Center,
        )],
    );
    let output = layout_tree(&tree, align_bounds);
    shared::clamp_rect_to_bounds(
        shared::rect_for(&output.rects, DRAG_OVERLAY_ID, shared::empty_rect(content)),
        Rect::from_min_max(
            Point::new(content.min.x, min_y),
            Point::new(content.max.x, max_y),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    #[test]
    fn drag_overlay_stays_above_status_bar() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let content = Rect::from_min_max(
            crate::gui::types::Point::new(260.0, 60.0),
            crate::gui::types::Point::new(1240.0, 640.0),
        );
        let status = Rect::from_min_max(
            crate::gui::types::Point::new(20.0, 660.0),
            crate::gui::types::Point::new(1260.0, 700.0),
        );
        let rect = compute_drag_overlay_rect(content, status, style.sizing);
        assert!(rect.min.y >= content.min.y);
        assert!(rect.max.y <= status.min.y - 1.0);
        assert!(rect.min.x >= content.min.x);
        assert!(rect.max.x <= content.max.x);
    }
}
