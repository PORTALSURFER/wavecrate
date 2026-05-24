use super::super::super::super::style::SizingTokens;
use super::filters::{browser_filter_cluster_width, compute_filter_control_side};
use crate::gui::types::Rect;

#[derive(Clone, Copy, Debug)]
pub(super) struct ToolbarLayoutMetrics {
    pub action_cluster_gap: f32,
    pub action_cluster_width: f32,
    pub action_side: f32,
    pub desired_search_width: f32,
    pub filter_gap: f32,
    pub filter_group_gap: f32,
    pub min_search_width: f32,
    max_filter_side: f32,
    outer_gap: f32,
}

impl ToolbarLayoutMetrics {
    pub(super) fn new(host: Rect, sizing: SizingTokens, available: f32, gap: f32) -> Self {
        let filter_gap = sizing.border_width.max(1.0) + 1.0;
        let action_side = (host.height() - (sizing.text_inset_y * 0.4))
            .floor()
            .clamp(14.0, 24.0)
            .min((available - gap).max(0.0));
        let action_cluster_gap = gap;
        let action_cluster_width = if action_side > 0.0 {
            (action_side * 2.0) + action_cluster_gap
        } else {
            0.0
        };

        Self {
            action_cluster_gap,
            action_cluster_width,
            action_side,
            desired_search_width: desired_search_width(host, sizing, available),
            filter_gap,
            filter_group_gap: filter_gap + sizing.border_width.max(1.0) + 2.0,
            min_search_width: sizing.browser_search_field_min_width.min(available),
            max_filter_side: (host.height() - (sizing.text_inset_y * 2.0))
                .floor()
                .clamp(6.0, 14.0),
            outer_gap: gap,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct FilterLayout {
    pub chip_side: f32,
    pub remaining_width: f32,
    pub total_width: f32,
}

impl FilterLayout {
    pub(super) fn fit(available: f32, metrics: ToolbarLayoutMetrics) -> Self {
        let mut layout = Self::for_search_width(available, metrics.desired_search_width, metrics);
        if layout.remaining_width < metrics.min_search_width
            && metrics.desired_search_width > metrics.min_search_width
        {
            layout = Self::for_search_width(available, metrics.min_search_width, metrics);
        }
        layout
    }

    fn for_search_width(available: f32, search_width: f32, metrics: ToolbarLayoutMetrics) -> Self {
        let chip_side = compute_filter_control_side(
            (available - search_width - metrics.action_cluster_width - (metrics.outer_gap * 2.0))
                .max(0.0),
            metrics.max_filter_side,
            metrics.filter_gap,
            metrics.filter_group_gap,
        );
        let total_width =
            browser_filter_cluster_width(chip_side, metrics.filter_gap, metrics.filter_group_gap)
                .min(available);
        let extra_chip_width = if chip_side > 0.0 {
            (chip_side + metrics.filter_group_gap) * 2.0
        } else {
            0.0
        };
        let remaining_width = (available
            - total_width
            - extra_chip_width
            - metrics.action_cluster_width
            - (metrics.outer_gap * 2.0))
            .max(0.0);

        Self {
            chip_side,
            remaining_width,
            total_width,
        }
    }
}

fn desired_search_width(host: Rect, sizing: SizingTokens, available: f32) -> f32 {
    ((host.width() * sizing.browser_search_field_ratio).max(sizing.browser_search_field_min_width))
        .min(
            (available * sizing.browser_search_field_ratio)
                .max(sizing.browser_search_field_min_width),
        )
}
