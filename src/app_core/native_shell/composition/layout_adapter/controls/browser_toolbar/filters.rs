use super::super::shared::{center_square_rect, empty_rect, layout_left_aligned_fixed_widths};
use super::{
    DERIVED_LABEL_FILTER_CHIP_COUNT, MARKED_FILTER_CHIP_COUNT, PLAYBACK_AGE_FILTER_CHIP_COUNT,
    RATING_FILTER_CHIP_COUNT, TOOLBAR_FILTER_CHIP_BASE_ID, TOOLBAR_FILTER_ID,
};
use crate::gui::types::{Point, Rect};

pub(super) fn compute_filter_control_side(
    available_width: f32,
    max_filter_side: f32,
    filter_gap: f32,
    filter_group_gap: f32,
) -> f32 {
    if available_width <= 0.0 {
        return 0.0;
    }

    let chip_count = (RATING_FILTER_CHIP_COUNT
        + PLAYBACK_AGE_FILTER_CHIP_COUNT
        + MARKED_FILTER_CHIP_COUNT
        + DERIVED_LABEL_FILTER_CHIP_COUNT) as f32;
    let intra_group_gap_count = (RATING_FILTER_CHIP_COUNT.saturating_sub(1)
        + PLAYBACK_AGE_FILTER_CHIP_COUNT.saturating_sub(1)) as f32;
    let raw_side =
        (available_width - (filter_gap * intra_group_gap_count) - (filter_group_gap * 2.0))
            / chip_count;

    if raw_side <= 0.0 {
        0.0
    } else {
        raw_side.floor().clamp(6.0, max_filter_side)
    }
}

pub(super) fn browser_filter_cluster_width(chip_side: f32, gap: f32, group_gap: f32) -> f32 {
    if chip_side <= 0.0 {
        return 0.0;
    }

    rating_filter_strip_width(chip_side, gap)
        + group_gap
        + playback_age_filter_strip_width(chip_side, gap)
}

pub(super) fn compute_rating_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
    first_chip_id: u64,
) -> [Rect; RATING_FILTER_CHIP_COUNT] {
    let empty = empty_rect(strip);
    if strip.width() <= 1.0 || strip.height() <= 0.0 || chip_side <= 0.0 {
        return [empty; RATING_FILTER_CHIP_COUNT];
    }

    let widths = [chip_side; RATING_FILTER_CHIP_COUNT];
    let rects = layout_left_aligned_fixed_widths(
        strip,
        gap,
        &widths,
        TOOLBAR_FILTER_ID + 10,
        first_chip_id,
    );
    let mut chips = [empty; RATING_FILTER_CHIP_COUNT];
    for (index, rect) in rects.into_iter().take(RATING_FILTER_CHIP_COUNT).enumerate() {
        chips[index] = center_square_rect(rect, chip_side);
    }
    chips
}

pub(super) fn compute_playback_age_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
    group_gap: f32,
) -> [Rect; PLAYBACK_AGE_FILTER_CHIP_COUNT] {
    let empty = empty_rect(strip);
    if strip.width() <= 1.0 || strip.height() <= 0.0 || chip_side <= 0.0 {
        return [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    }

    let widths = [chip_side; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    let rating_strip_width = rating_filter_strip_width(chip_side, gap);
    let age_strip = Rect::from_min_max(
        Point::new(
            (strip.min.x + rating_strip_width + group_gap).min(strip.max.x),
            strip.min.y,
        ),
        strip.max,
    );
    let rects = layout_left_aligned_fixed_widths(
        age_strip,
        gap,
        &widths,
        TOOLBAR_FILTER_ID + 20,
        TOOLBAR_FILTER_CHIP_BASE_ID + RATING_FILTER_CHIP_COUNT as u64,
    );
    let mut chips = [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    for (index, rect) in rects
        .into_iter()
        .take(PLAYBACK_AGE_FILTER_CHIP_COUNT)
        .enumerate()
    {
        chips[index] = center_square_rect(rect, chip_side);
    }
    chips
}

fn rating_filter_strip_width(chip_side: f32, gap: f32) -> f32 {
    if chip_side <= 0.0 {
        return 0.0;
    }

    (chip_side * RATING_FILTER_CHIP_COUNT as f32)
        + (gap * (RATING_FILTER_CHIP_COUNT.saturating_sub(1) as f32))
}

fn playback_age_filter_strip_width(chip_side: f32, gap: f32) -> f32 {
    if chip_side <= 0.0 {
        return 0.0;
    }

    (chip_side * PLAYBACK_AGE_FILTER_CHIP_COUNT as f32)
        + (gap * (PLAYBACK_AGE_FILTER_CHIP_COUNT.saturating_sub(1) as f32))
}
