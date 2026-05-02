#![allow(dead_code)]
use super::super::super::style::SizingTokens;
use super::shared::{
    center_square_rect, clamp_rect_to_bounds, empty_rect, layout_left_aligned_fixed_widths,
};
use crate::gui::types::{Point, Rect};

const TOOLBAR_FILTER_ID: u64 = 801;
const TOOLBAR_FILTER_CHIP_BASE_ID: u64 = 820;
const RATING_FILTER_CHIP_COUNT: usize = 8;
const PLAYBACK_AGE_FILTER_CHIP_COUNT: usize = 3;
const MARKED_FILTER_CHIP_COUNT: usize = 1;
const DERIVED_LABEL_FILTER_CHIP_COUNT: usize = 1;

/// Slot-resolved browser toolbar sections for search and chip controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarSections {
    pub rating_filter_chips: [Rect; 8],
    pub playback_age_filter_chips: [Rect; 3],
    pub marked_filter_chip: Rect,
    pub derived_label_filter_chip: Rect,
    pub action_slots: [Rect; 3],
    pub search_field: Rect,
    pub activity_chip: Rect,
    pub sort_chip: Rect,
    pub triage_chips: [Rect; 3],
}

/// Compute browser toolbar search/activity/sort partitions from slot rows.
pub(crate) fn compute_browser_toolbar_sections(
    toolbar: Rect,
    sizing: SizingTokens,
) -> BrowserToolbarSections {
    let empty = empty_rect(toolbar);
    let empty_chips = [empty; 3];
    let empty_filter_chips = [empty; RATING_FILTER_CHIP_COUNT];
    let empty_playback_age_filter_chips = [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    let empty_action_slots = [empty; 3];
    if toolbar.width() <= 0.0 || toolbar.height() <= 0.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            derived_label_filter_chip: empty,
            action_slots: empty_action_slots,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let gap = sizing.action_button_gap.max(1.0);
    let host = toolbar;
    if host.width() <= 1.0 || host.height() <= 0.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            derived_label_filter_chip: empty,
            action_slots: empty_action_slots,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let left_min = host.min.x + sizing.text_inset_x;
    let left_max = (host.max.x - sizing.text_inset_x).max(left_min);
    let available = (left_max - left_min).max(0.0);
    if available <= 1.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            derived_label_filter_chip: empty,
            action_slots: empty_action_slots,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let filter_gap = sizing.border_width.max(1.0) + 1.0;
    let filter_group_gap = filter_gap + sizing.border_width.max(1.0) + 2.0;
    let max_filter_side = (host.height() - (sizing.text_inset_y * 2.0))
        .floor()
        .clamp(6.0, 14.0);
    let desired_search_width = ((host.width() * sizing.browser_search_field_ratio)
        .max(sizing.browser_search_field_min_width))
    .min(
        (available * sizing.browser_search_field_ratio).max(sizing.browser_search_field_min_width),
    );
    let action_side = (host.height() - (sizing.text_inset_y * 0.4))
        .floor()
        .clamp(14.0, 24.0)
        .min((available - gap).max(0.0));
    let action_button_count = 2usize;
    let action_cluster_gap = gap;
    let action_cluster_width = if action_side > 0.0 {
        (action_side * action_button_count as f32)
            + (action_cluster_gap * action_button_count.saturating_sub(1) as f32)
    } else {
        0.0
    };
    let min_search_width = sizing.browser_search_field_min_width.min(available);
    let mut filter_side = compute_filter_control_side(
        (available - desired_search_width - action_cluster_width - (gap * 2.0)).max(0.0),
        max_filter_side,
        filter_gap,
        filter_group_gap,
    );
    let mut filter_total_width =
        browser_filter_cluster_width(filter_side, filter_gap, filter_group_gap).min(available);
    let marked_chip_side = filter_side.max(0.0);
    let marked_chip_width = if marked_chip_side > 0.0 {
        marked_chip_side + filter_group_gap
    } else {
        0.0
    };
    let derived_label_chip_width = marked_chip_width;
    let mut remaining_after_filters = (available
        - filter_total_width
        - marked_chip_width
        - derived_label_chip_width
        - action_cluster_width
        - (gap * 2.0))
        .max(0.0);
    if remaining_after_filters < min_search_width && desired_search_width > min_search_width {
        filter_side = compute_filter_control_side(
            (available - min_search_width - action_cluster_width - (gap * 2.0)).max(0.0),
            max_filter_side,
            filter_gap,
            filter_group_gap,
        );
        filter_total_width =
            browser_filter_cluster_width(filter_side, filter_gap, filter_group_gap).min(available);
        remaining_after_filters = (available
            - filter_total_width
            - if filter_side > 0.0 {
                (filter_side + filter_group_gap) * 2.0
            } else {
                0.0
            }
            - action_cluster_width
            - (gap * 2.0))
            .max(0.0);
    }
    let right_tag_width = (action_side * 2.4).clamp(44.0, 72.0).min(available);
    let right_tag_slot = if action_side > 0.0 && right_tag_width > 0.0 {
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new((left_max - right_tag_width).max(left_min), host.min.y),
                Point::new(left_max, host.max.y),
            ),
            host,
        )
    } else {
        empty
    };
    let search_right_edge = if right_tag_slot.width() > 1.0 {
        right_tag_slot.min.x - gap
    } else {
        left_max
    };
    let search_width = desired_search_width
        .min(remaining_after_filters.max(min_search_width))
        .max(0.0);
    let filter_bounds = Rect::from_min_max(
        Point::new(left_min, host.min.y),
        Point::new((left_min + filter_total_width).min(left_max), host.max.y),
    );
    let filter_strip = if filter_total_width > 0.0 {
        clamp_rect_to_bounds(filter_bounds, host)
    } else {
        empty
    };
    let search_field = if search_width > 0.0 {
        Rect::from_min_max(
            Point::new((search_right_edge - search_width).max(left_min), host.min.y),
            Point::new(search_right_edge.max(left_min), host.max.y),
        )
    } else {
        empty
    };
    let marked_filter_chip = if marked_chip_side > 0.0 {
        let min_x = (filter_strip.max.x + filter_group_gap).min(left_max);
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(min_x, host.min.y),
                Point::new((min_x + marked_chip_side).min(left_max), host.max.y),
            ),
            host,
        )
    } else {
        empty
    };
    let derived_label_filter_chip = if marked_chip_side > 0.0 {
        let min_x = if marked_filter_chip.width() > 1.0 {
            (marked_filter_chip.max.x + filter_group_gap).min(left_max)
        } else {
            (filter_strip.max.x + filter_group_gap).min(left_max)
        };
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(min_x, host.min.y),
                Point::new((min_x + marked_chip_side).min(left_max), host.max.y),
            ),
            host,
        )
    } else {
        empty
    };
    let controls_right_edge = if derived_label_filter_chip.width() > 1.0 {
        derived_label_filter_chip.max.x
    } else if marked_filter_chip.width() > 1.0 {
        marked_filter_chip.max.x
    } else {
        filter_strip.max.x
    };
    let action_cluster = if action_side > 0.0 && search_field.width() > 1.0 {
        let action_max_x = (search_field.min.x - gap).max(left_min);
        Rect::from_min_max(
            Point::new(
                (action_max_x - action_cluster_width).max(controls_right_edge + gap),
                host.min.y,
            ),
            Point::new(action_max_x, host.max.y),
        )
    } else {
        empty
    };
    let mut action_slots = compute_action_slot_rects(
        clamp_rect_to_bounds(action_cluster, host),
        action_side,
        action_cluster_gap,
    );
    action_slots[2] = right_tag_slot;
    let rating_filter_chips = compute_rating_filter_chip_rects(
        filter_strip,
        filter_side,
        filter_gap,
        TOOLBAR_FILTER_CHIP_BASE_ID,
    );
    let playback_age_filter_chips = compute_playback_age_filter_chip_rects(
        filter_strip,
        filter_side,
        filter_gap,
        filter_group_gap,
    );
    BrowserToolbarSections {
        rating_filter_chips,
        playback_age_filter_chips,
        marked_filter_chip,
        derived_label_filter_chip,
        action_slots,
        search_field,
        activity_chip: empty,
        sort_chip: empty,
        triage_chips: empty_chips,
    }
}

fn compute_filter_control_side(
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

fn browser_filter_cluster_width(chip_side: f32, gap: f32, group_gap: f32) -> f32 {
    if chip_side <= 0.0 {
        return 0.0;
    }
    rating_filter_strip_width(chip_side, gap)
        + group_gap
        + playback_age_filter_strip_width(chip_side, gap)
}

fn compute_action_slot_rects(cluster: Rect, action_side: f32, gap: f32) -> [Rect; 3] {
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

fn compute_rating_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
    first_chip_id: u64,
) -> [Rect; 8] {
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

fn compute_playback_age_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
    group_gap: f32,
) -> [Rect; 3] {
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
