use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRatingIndicatorLayout {
    pub(in crate::gui::native_shell::state) rects: [Rect; 3],
    pub(in crate::gui::native_shell::state) count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRatingIndicatorAnchor {
    pub(in crate::gui::native_shell::state) sample_label: Rect,
    pub(in crate::gui::native_shell::state) label_origin_x: f32,
    pub(in crate::gui::native_shell::state) label_rendered_width: f32,
    pub(in crate::gui::native_shell::state) right_limit_x: f32,
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_reserved_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let count = browser_rating_indicator_count(rating_level, locked);
    if count == 0 {
        return 0.0;
    }
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing);
    let gap = browser_rating_indicator_gap(sizing);
    let text_gap = browser_rating_indicator_text_gap(sizing);
    (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap) + text_gap
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_layout(
    anchor: BrowserRatingIndicatorAnchor,
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> Option<BrowserRatingIndicatorLayout> {
    let count = browser_rating_indicator_count(rating_level, locked);
    let sample_label = anchor.sample_label;
    if count == 0 || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return None;
    }
    let side = browser_rating_indicator_side(sizing).min(sample_label.height().max(1.0));
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing)
        .min(sample_label.width().max(1.0));
    let gap = browser_rating_indicator_gap(sizing);
    let total_width = (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap);
    let ideal_start_x = anchor.label_origin_x
        + anchor.label_rendered_width.max(0.0)
        + browser_rating_indicator_text_gap(sizing);
    let right_limit_x = anchor
        .right_limit_x
        .clamp(sample_label.min.x, sample_label.max.x);
    let max_start_x = (right_limit_x - total_width).max(sample_label.min.x);
    let start_x = ideal_start_x.clamp(sample_label.min.x, max_start_x);
    let min_y = sample_label.min.y + ((sample_label.height() - side) * 0.5).floor();
    let max_y = (min_y + side).min(sample_label.max.y);
    let mut rects = [Rect::from_min_max(sample_label.min, sample_label.min); 3];
    for (index, rect) in rects.iter_mut().take(count).enumerate() {
        let min_x = start_x + index as f32 * (width + gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + width).min(sample_label.max.x), max_y),
        );
    }
    Some(BrowserRatingIndicatorLayout { rects, count })
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_color(
    style: &StyleTokens,
    rating_level: i8,
) -> Rgba8 {
    if rating_level < 0 {
        style.accent_trash
    } else {
        style.accent_mint
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_side(
    sizing: SizingTokens,
) -> f32 {
    (sizing.font_meta * 0.68).round().clamp(5.0, 8.0)
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_count(
    rating_level: i8,
    locked: bool,
) -> usize {
    if locked && rating_level > 0 {
        1
    } else {
        rating_level.unsigned_abs().min(3) as usize
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_unit_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let side = browser_rating_indicator_side(sizing);
    if locked && rating_level > 0 {
        (side * 2.4).round().max(side + 2.0)
    } else {
        side
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 1.0
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_text_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(2.0)
}
