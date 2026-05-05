use super::*;
use crate::gui::badge::{
    InlineBadgeMetrics, inline_badge_cluster_reserved_width, inline_badge_labels_owned,
    inline_badge_rects_for_labels, inline_badge_text_origin,
};

const INLINE_METADATA_DELIMITER: &str = " · ";

fn browser_inline_metadata_metrics(sizing: SizingTokens) -> InlineBadgeMetrics {
    InlineBadgeMetrics::new(
        sizing.font_meta,
        browser_inline_metadata_chip_padding_x(sizing),
        browser_inline_metadata_chip_padding_y(sizing),
        browser_inline_metadata_chip_gap(sizing),
        browser_inline_metadata_gap(sizing),
        10.0,
    )
}

/// Return the horizontal gap between an item label and its inline metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_metadata_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(6.0).max(3.0)
}

/// Materialize inline browser metadata labels once when a cache boundary owns them.
pub(in crate::gui::native_shell::state) fn browser_inline_metadata_labels_owned(
    text: &str,
) -> Vec<String> {
    inline_badge_labels_owned(text, INLINE_METADATA_DELIMITER)
}

/// Return reserved width for a pre-split inline metadata cluster.
pub(in crate::gui::native_shell::state) fn browser_inline_metadata_reserved_width_for_labels(
    labels: &[String],
    sizing: SizingTokens,
) -> f32 {
    inline_badge_cluster_reserved_width(labels, browser_inline_metadata_metrics(sizing))
}

/// Compute chip rects for pre-split inline browser metadata labels.
pub(in crate::gui::native_shell::state) fn browser_inline_metadata_chip_rects_for_labels(
    item_label: Rect,
    labels: &[String],
    trailing_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    inline_badge_rects_for_labels(
        item_label,
        labels,
        trailing_reserved_width,
        browser_inline_metadata_metrics(sizing),
    )
}

/// Return the inset text origin for one inline browser metadata chip.
pub(in crate::gui::native_shell::state) fn browser_inline_metadata_text_origin(
    chip_rect: Rect,
    sizing: SizingTokens,
) -> Point {
    inline_badge_text_origin(chip_rect, browser_inline_metadata_metrics(sizing))
}

pub(in crate::gui::native_shell::state) fn browser_inline_metadata_chip_padding_x(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_metadata_chip_padding_y(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_y.min(3.0).max(1.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_metadata_chip_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 2.0
}
