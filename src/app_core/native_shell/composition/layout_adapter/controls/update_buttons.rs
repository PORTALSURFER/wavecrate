use super::super::super::style::SizingTokens;
use super::shared::{layout_right_aligned_fixed_widths, visible_suffix_widths};
use crate::gui::types::{Point, Rect};

const UPDATE_BUTTON_ROW_ID: u64 = 700;
const UPDATE_BUTTON_SPACER_ID: u64 = 701;
const UPDATE_BUTTON_BASE_ID: u64 = 710;

/// Compute top-bar update action button rects aligned to the right cluster.
pub(crate) fn compute_update_action_button_rects(
    row: Rect,
    action_cluster: Rect,
    sizing: SizingTokens,
    labels: &[&str],
) -> Vec<Rect> {
    if labels.is_empty() || row.width() <= 0.0 || row.height() <= 0.0 {
        return Vec::new();
    }
    let gap = sizing.action_button_gap.max(1.0);
    let button_height = (row.height() - (sizing.text_inset_y * 0.4))
        .max(12.0)
        .min(row.height());
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| {
            ((*label).chars().count() as f32 * (sizing.font_meta * 0.62)
                + (sizing.text_inset_x * 2.0))
                .clamp(42.0, 84.0)
        })
        .collect();
    let available_width =
        ((action_cluster.max.x - sizing.text_inset_x) - action_cluster.min.x).max(0.0);
    let visible_widths = visible_suffix_widths(&widths, available_width, gap);
    if visible_widths.is_empty() {
        return Vec::new();
    }
    let y = row.min.y + ((row.height() - button_height) * 0.5);
    let bounds = Rect::from_min_max(
        Point::new(action_cluster.min.x, y),
        Point::new(
            (action_cluster.max.x - sizing.text_inset_x).max(action_cluster.min.x),
            (y + button_height).min(row.max.y),
        ),
    );
    layout_right_aligned_fixed_widths(
        bounds,
        gap,
        &visible_widths,
        UPDATE_BUTTON_ROW_ID,
        UPDATE_BUTTON_SPACER_ID,
        UPDATE_BUTTON_BASE_ID,
    )
}
