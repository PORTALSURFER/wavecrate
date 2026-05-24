#![allow(dead_code)]
#[path = "browser_toolbar/actions.rs"]
mod actions;
#[path = "browser_toolbar/filters.rs"]
mod filters;
#[path = "browser_toolbar/metrics.rs"]
mod metrics;

use self::actions::compute_action_slot_rects;
use self::filters::{compute_playback_age_filter_chip_rects, compute_rating_filter_chip_rects};
use self::metrics::{FilterLayout, ToolbarLayoutMetrics};
use super::super::super::style::SizingTokens;
use super::shared::{clamp_rect_to_bounds, empty_rect};
use crate::gui::types::{Point, Rect};

pub(super) const TOOLBAR_FILTER_ID: u64 = 801;
pub(super) const TOOLBAR_FILTER_CHIP_BASE_ID: u64 = 820;
pub(super) const RATING_FILTER_CHIP_COUNT: usize = 8;
pub(super) const PLAYBACK_AGE_FILTER_CHIP_COUNT: usize = 3;
pub(super) const MARKED_FILTER_CHIP_COUNT: usize = 1;
pub(super) const DERIVED_LABEL_FILTER_CHIP_COUNT: usize = 1;

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
    if toolbar.width() <= 0.0 || toolbar.height() <= 0.0 {
        return empty_sections(empty);
    }
    let gap = sizing.action_button_gap.max(1.0);
    let host = toolbar;
    if host.width() <= 1.0 || host.height() <= 0.0 {
        return empty_sections(empty);
    }
    let left_min = host.min.x + sizing.text_inset_x;
    let left_max = (host.max.x - sizing.text_inset_x).max(left_min);
    let available = (left_max - left_min).max(0.0);
    if available <= 1.0 {
        return empty_sections(empty);
    }
    let metrics = ToolbarLayoutMetrics::new(host, sizing, available, gap);
    let filter_layout = FilterLayout::fit(available, metrics);
    let right_tag_width = (metrics.action_side * 2.4).clamp(44.0, 72.0).min(available);
    let right_tag_slot = if metrics.action_side > 0.0 && right_tag_width > 0.0 {
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
    let search_width = metrics
        .desired_search_width
        .min(filter_layout.remaining_width.max(metrics.min_search_width))
        .max(0.0);
    let filter_bounds = Rect::from_min_max(
        Point::new(left_min, host.min.y),
        Point::new(
            (left_min + filter_layout.total_width).min(left_max),
            host.max.y,
        ),
    );
    let filter_strip = if filter_layout.total_width > 0.0 {
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
    let marked_filter_chip = if filter_layout.chip_side > 0.0 {
        let min_x = (filter_strip.max.x + metrics.filter_group_gap).min(left_max);
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(min_x, host.min.y),
                Point::new((min_x + filter_layout.chip_side).min(left_max), host.max.y),
            ),
            host,
        )
    } else {
        empty
    };
    let derived_label_filter_chip = if filter_layout.chip_side > 0.0 {
        let min_x = if marked_filter_chip.width() > 1.0 {
            (marked_filter_chip.max.x + metrics.filter_group_gap).min(left_max)
        } else {
            (filter_strip.max.x + metrics.filter_group_gap).min(left_max)
        };
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(min_x, host.min.y),
                Point::new((min_x + filter_layout.chip_side).min(left_max), host.max.y),
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
    let action_cluster = if metrics.action_side > 0.0 && search_field.width() > 1.0 {
        let action_max_x = (search_field.min.x - gap).max(left_min);
        Rect::from_min_max(
            Point::new(
                (action_max_x - metrics.action_cluster_width).max(controls_right_edge + gap),
                host.min.y,
            ),
            Point::new(action_max_x, host.max.y),
        )
    } else {
        empty
    };
    let mut action_slots = compute_action_slot_rects(
        clamp_rect_to_bounds(action_cluster, host),
        metrics.action_side,
        metrics.action_cluster_gap,
    );
    action_slots[2] = right_tag_slot;
    let rating_filter_chips = compute_rating_filter_chip_rects(
        filter_strip,
        filter_layout.chip_side,
        metrics.filter_gap,
        TOOLBAR_FILTER_CHIP_BASE_ID,
    );
    let playback_age_filter_chips = compute_playback_age_filter_chip_rects(
        filter_strip,
        filter_layout.chip_side,
        metrics.filter_gap,
        metrics.filter_group_gap,
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
        triage_chips: [empty; 3],
    }
}

fn empty_sections(empty: Rect) -> BrowserToolbarSections {
    BrowserToolbarSections {
        rating_filter_chips: [empty; RATING_FILTER_CHIP_COUNT],
        playback_age_filter_chips: [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT],
        marked_filter_chip: empty,
        derived_label_filter_chip: empty,
        action_slots: [empty; 3],
        search_field: empty,
        activity_chip: empty,
        sort_chip: empty,
        triage_chips: [empty; 3],
    }
}
