use super::super::super::style::SizingTokens;
use super::shared::{clamp_rect_right_edge, layout_right_aligned_fixed_widths};
use crate::gui::types::{Point, Rect};

const SIDEBAR_BUTTON_ROW_ID: u64 = 770;
const SIDEBAR_BUTTON_SPACER_ID: u64 = 771;
const SIDEBAR_BUTTON_BASE_ID: u64 = 780;

/// Compute sidebar footer action button rects aligned to the right edge.
pub(crate) fn compute_sidebar_action_button_rects(
    footer: Rect,
    sizing: SizingTokens,
    button_count: usize,
) -> Vec<Rect> {
    if button_count == 0 || footer.width() <= 0.0 || footer.height() <= 0.0 {
        return Vec::new();
    }
    let gap = sizing.sidebar_action_button_gap;
    let available_width = (footer.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    let button_count_f32 = button_count as f32;
    let button_width = if button_count_f32 > 0.0 {
        ((available_width - (gap * (button_count_f32 - 1.0)).max(0.0)).max(0.0) / button_count_f32)
            .min(sizing.sidebar_action_button_width)
    } else {
        sizing.sidebar_action_button_width
    };
    let button_height = sizing
        .sidebar_action_button_height
        .min((footer.height() - 1.0).max(1.0));
    let y_min = footer.min.y + 1.0;
    let y_max = (footer.max.y - button_height).max(y_min);
    let y = (footer.max.y - button_height - sizing.text_inset_y)
        .max(y_min)
        .min(y_max);
    let bounds = Rect::from_min_max(
        Point::new(footer.min.x + sizing.text_inset_x, y),
        Point::new(
            (footer.max.x - sizing.text_inset_x).max(footer.min.x),
            (y + button_height).min(footer.max.y),
        ),
    );
    let widths = vec![button_width; button_count];
    layout_right_aligned_fixed_widths(
        bounds,
        gap,
        &widths,
        SIDEBAR_BUTTON_ROW_ID,
        SIDEBAR_BUTTON_SPACER_ID,
        SIDEBAR_BUTTON_BASE_ID,
    )
    .into_iter()
    .map(|rect| clamp_rect_right_edge(rect, footer, footer.max.x - 1.0))
    .collect()
}
