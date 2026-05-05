use super::{SizingTokens, tier_deltas::sizing_for_tier};
use crate::gui::native_shell::style::LayoutScaleTier;

fn assert_common_invariants(sizing: SizingTokens) {
    assert!(sizing.min_viewport_width > 0.0);
    assert!(sizing.min_viewport_height > 0.0);
    assert!(sizing.sidebar_min_width <= sizing.sidebar_max_width);
    assert!(sizing.waveform_min_height <= sizing.waveform_max_height);
    assert!(sizing.browser_footer_min_height <= sizing.browser_footer_max_height);
    assert!(sizing.source_rows_min_when_split <= sizing.source_rows_max);
    assert!(sizing.tree_rows_min <= sizing.tree_rows_max);
    assert!(sizing.browser_toolbar_min_height <= sizing.browser_toolbar_height);
    assert!(sizing.browser_table_header_min_height <= sizing.browser_table_header_height);
    assert!(sizing.browser_tabs_min_height <= sizing.browser_tabs_height);
    assert!(sizing.prompt_width > sizing.overlay_button_width);
    assert!(sizing.prompt_min_height > sizing.overlay_button_height);
}

#[test]
fn sizing_tiers_preserve_core_bounds_and_caps() {
    for tier in [
        LayoutScaleTier::Compact,
        LayoutScaleTier::Standard,
        LayoutScaleTier::Wide,
    ] {
        assert_common_invariants(sizing_for_tier(tier));
    }
}

#[test]
fn sizing_tiers_keep_expected_density_ordering() {
    let compact = sizing_for_tier(LayoutScaleTier::Compact);
    let standard = sizing_for_tier(LayoutScaleTier::Standard);
    let wide = sizing_for_tier(LayoutScaleTier::Wide);

    assert!(compact.source_rows_max <= standard.source_rows_max);
    assert!(standard.source_rows_max <= wide.source_rows_max);
    assert!(compact.tree_rows_max <= standard.tree_rows_max);
    assert!(standard.tree_rows_max <= wide.tree_rows_max);
    assert!(compact.browser_rows_max_per_column <= standard.browser_rows_max_per_column);
    assert!(standard.browser_rows_max_per_column <= wide.browser_rows_max_per_column);
    assert!(compact.recovery_badge_min_width <= standard.recovery_badge_min_width);
    assert!(standard.recovery_badge_min_width <= wide.recovery_badge_min_width);
}

#[test]
fn ui_scaling_preserves_zero_gap_shell_contract() {
    let scaled = sizing_for_tier(LayoutScaleTier::Wide).with_ui_scale(2.0);
    assert_eq!(scaled.frame_inset, 0.0);
    assert_eq!(scaled.panel_gap, 0.0);
}
