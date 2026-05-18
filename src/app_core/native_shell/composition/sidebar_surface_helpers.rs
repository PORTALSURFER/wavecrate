//! Shared sizing helpers for sidebar chrome surfaces.

use crate::app_core::native_shell::composition::style::SizingTokens;

/// Return the canonical square sidebar-header add-button edge length.
pub(super) fn header_button_side(sizing: SizingTokens) -> f32 {
    (sizing.font_header + (sizing.text_inset_y * 1.5))
        .round()
        .max(12.0)
}

/// Return the bounded per-button footer width for the current sidebar band.
pub(super) fn footer_action_button_width(
    footer_width: f32,
    sizing: SizingTokens,
    button_count: usize,
) -> f32 {
    if button_count == 0 {
        return 0.0;
    }
    let gap = sizing.sidebar_action_button_gap.max(0.0);
    let available_width = (footer_width - (sizing.text_inset_x * 2.0)).max(0.0);
    ((available_width - (gap * button_count.saturating_sub(1) as f32)).max(0.0)
        / button_count as f32)
        .min(sizing.sidebar_action_button_width)
        .max(1.0)
}
