use super::super::shared::{center_square_rect, empty_rect, layout_left_aligned_fixed_widths};
use super::TOOLBAR_FILTER_ID;
use crate::gui::types::Rect;

pub(super) fn compute_action_slot_rects(cluster: Rect, action_side: f32, gap: f32) -> [Rect; 3] {
    let empty = empty_rect(cluster);
    if cluster.width() <= 1.0 || cluster.height() <= 0.0 || action_side <= 0.0 {
        return [empty; 3];
    }

    let widths = [action_side; 3];
    let rects = layout_left_aligned_fixed_widths(cluster, gap, &widths, TOOLBAR_FILTER_ID + 30, 0);
    let mut slots = [empty; 3];
    for (index, rect) in rects.into_iter().take(3).enumerate() {
        slots[index] = center_square_rect(rect, action_side);
    }
    slots
}
