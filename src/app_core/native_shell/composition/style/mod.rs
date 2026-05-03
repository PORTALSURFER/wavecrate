//! Shared style tokens for the native shell renderer.

use std::ops::Deref;

use crate::theme::{self, ThemeTokens, ViewportScaleTier};

/// Compatibility-only shell chrome tokens that do not belong in core theming.
mod chrome;
/// Semantic color tokens used by the retained shell paint pass.
mod palette;
/// Geometry and typography sizing tokens plus UI-scale inflation rules.
mod sizing;

pub(crate) use chrome::ShellChromeTokens;
pub(crate) use sizing::SizingTokens;
pub(crate) use theme::ViewportScaleTier as LayoutScaleTier;

/// Style tokens consumed by the retained shell paint pass.
///
/// Generic colors and motion live in [`ThemeTokens`]. Shell-only sidebar chrome
/// stays in [`ShellChromeTokens`], which makes the compatibility boundary
/// explicit while preserving the existing compatibility-shell behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StyleTokens {
    /// Viewport scale tier used to derive the token set.
    pub layout_tier: LayoutScaleTier,
    /// Generic reusable theme tokens for widgets, containers, and runtimes.
    pub theme: ThemeTokens,
    /// Shell-only chrome tokens that remain behind the compatibility boundary.
    pub chrome: ShellChromeTokens,
    /// Compact sizing tokens for layout rhythm and element scale.
    pub sizing: SizingTokens,
}

impl Deref for StyleTokens {
    type Target = ThemeTokens;

    fn deref(&self) -> &Self::Target {
        &self.theme
    }
}

impl Default for StyleTokens {
    fn default() -> Self {
        Self::for_viewport_width(1280.0)
    }
}

impl StyleTokens {
    /// Build style tokens tuned for a viewport width and DPI scale factor.
    ///
    /// The input scale factor is clamped to a safe range so accidental outlier
    /// values cannot collapse or overinflate layout geometry. A small baseline
    /// multiplier is then applied to keep the default UI density slightly larger.
    pub(crate) fn for_viewport_with_scale(viewport_width: f32, ui_scale: f32) -> Self {
        let mut tokens = Self::for_tier(ViewportScaleTier::from_viewport_width(viewport_width));
        tokens.sizing = tokens
            .sizing
            .with_ui_scale(theme::effective_ui_scale(ui_scale));
        tokens
    }

    /// Build style tokens tuned for a viewport width tier.
    pub(crate) fn for_viewport_width(viewport_width: f32) -> Self {
        Self::for_viewport_with_scale(viewport_width, 1.0)
    }

    /// Build style tokens for an explicit scale tier.
    pub(crate) fn for_tier(layout_tier: LayoutScaleTier) -> Self {
        let (mut theme, chrome) = palette::tokens_for_tier(layout_tier);
        let motion = theme::visual_policy_for_tier(layout_tier);
        theme.state_hover_soft = motion.state_hover_soft;
        theme.state_hover_strong = motion.state_hover_strong;
        theme.state_selected_blend = motion.state_selected_blend;
        theme.state_focus_pulse_blend = motion.state_focus_pulse_blend;
        theme.scrim_soft_alpha = motion.scrim_soft_alpha;
        theme.scrim_modal_alpha = motion.scrim_modal_alpha;
        theme.motion_speed_transport = motion.motion_speed_transport;
        theme.motion_speed_idle = motion.motion_speed_idle;
        theme.motion_focus_wave_amp = motion.motion_focus_wave_amp;
        theme.motion_focus_text_wave_amp = motion.motion_focus_text_wave_amp;
        Self {
            layout_tier,
            theme,
            chrome,
            sizing: sizing::sizing_for_tier(layout_tier),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutScaleTier, StyleTokens};
    use crate::theme::DEFAULT_UI_SCALE;

    #[test]
    fn viewport_width_maps_to_expected_tier() {
        assert_eq!(
            LayoutScaleTier::from_viewport_width(820.0),
            LayoutScaleTier::Compact
        );
        assert_eq!(
            LayoutScaleTier::from_viewport_width(1280.0),
            LayoutScaleTier::Standard
        );
        assert_eq!(
            LayoutScaleTier::from_viewport_width(2300.0),
            LayoutScaleTier::Wide
        );
    }

    #[test]
    fn explicit_tier_builder_matches_width_builder() {
        let compact = StyleTokens::for_tier(LayoutScaleTier::Compact).sizing;
        let compact_from_width = StyleTokens::for_viewport_width(820.0).sizing;
        assert_eq!(compact.with_ui_scale(DEFAULT_UI_SCALE), compact_from_width);

        let standard = StyleTokens::for_tier(LayoutScaleTier::Standard).sizing;
        let standard_from_width = StyleTokens::for_viewport_width(1280.0).sizing;
        assert_eq!(
            standard.with_ui_scale(DEFAULT_UI_SCALE),
            standard_from_width
        );

        let wide = StyleTokens::for_tier(LayoutScaleTier::Wide).sizing;
        let wide_from_width = StyleTokens::for_viewport_width(2300.0).sizing;
        assert_eq!(wide.with_ui_scale(DEFAULT_UI_SCALE), wide_from_width);
    }

    #[test]
    fn viewport_tiers_adjust_row_heights() {
        let narrow = StyleTokens::for_viewport_width(820.0);
        let standard = StyleTokens::for_viewport_width(1280.0);
        let wide = StyleTokens::for_viewport_width(2300.0);
        assert!(narrow.sizing.browser_row_height < standard.sizing.browser_row_height);
        assert!(standard.sizing.browser_row_height < wide.sizing.browser_row_height);
        assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
    }

    #[test]
    fn viewport_tiers_adjust_header_bands() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.column_header_block_height < wide.sizing.column_header_block_height);
        assert!(
            narrow.sizing.waveform_header_block_height < wide.sizing.waveform_header_block_height
        );
    }

    #[test]
    fn viewport_tiers_adjust_shell_frame_metrics() {
        let compact = StyleTokens::for_viewport_width(900.0);
        let standard = StyleTokens::for_viewport_width(1280.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(wide.sizing.top_bar_height >= standard.sizing.top_bar_height);
        assert!(wide.sizing.frame_inset >= standard.sizing.frame_inset);
        assert!(wide.sizing.column_gap >= standard.sizing.column_gap);
        assert!(compact.sizing.top_bar_height >= standard.sizing.top_bar_height);
    }

    #[test]
    fn viewport_tiers_adjust_render_row_caps() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.source_rows_max < wide.sizing.source_rows_max);
        assert!(narrow.sizing.tree_rows_max < wide.sizing.tree_rows_max);
        assert!(
            narrow.sizing.browser_rows_max_per_column < wide.sizing.browser_rows_max_per_column
        );
    }

    #[test]
    fn standard_tier_matches_classic_dense_shell_targets() {
        let standard = StyleTokens::for_viewport_width(1280.0);
        assert!((0.14..=0.18).contains(&standard.sizing.sidebar_ratio));
        assert!(
            (15.5 * DEFAULT_UI_SCALE..=17.0 * DEFAULT_UI_SCALE)
                .contains(&standard.sizing.browser_row_height)
        );
        assert!(standard.sizing.browser_tabs_height <= 20.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.browser_toolbar_height <= 21.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.browser_table_header_height <= 20.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.waveform_ratio <= 0.36);
        assert!(standard.sizing.sidebar_max_width <= 220.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.font_body <= 9.1 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.font_meta <= 8.8 * DEFAULT_UI_SCALE);
    }

    #[test]
    fn viewport_scale_preserves_tiers_and_inflates_geometry() {
        let scaled = StyleTokens::for_viewport_with_scale(1280.0, 1.5);
        let base = StyleTokens::for_viewport_width(1280.0);
        assert_eq!(scaled.layout_tier, base.layout_tier);
        assert_eq!(scaled.sizing.sidebar_ratio, base.sizing.sidebar_ratio);
        assert!((scaled.sizing.font_body - (base.sizing.font_body * 1.5)).abs() < 0.0001);
        assert!((scaled.sizing.top_bar_height - (base.sizing.top_bar_height * 1.5)).abs() < 0.0001);
        assert!(
            (scaled.sizing.action_button_width - (base.sizing.action_button_width * 1.5)).abs()
                < 0.0001
        );
        assert!(
            (scaled.sizing.sidebar_max_width - (base.sizing.sidebar_max_width * 1.5)).abs()
                < 0.0001
        );
    }

    #[test]
    fn viewport_scale_is_clamped() {
        let below = StyleTokens::for_viewport_with_scale(1280.0, 0.5);
        let identity = StyleTokens::for_viewport_with_scale(1280.0, 1.0);
        let above = StyleTokens::for_viewport_with_scale(1280.0, 4.0);
        let max = StyleTokens::for_viewport_with_scale(1280.0, 3.0);
        assert_eq!(below, identity);
        assert!((above.sizing.font_body - max.sizing.font_body).abs() < 0.0001);
    }

    #[test]
    fn viewport_tiers_adjust_interaction_density_tokens() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.focus_stroke_width < wide.sizing.focus_stroke_width);
        assert!(narrow.sizing.header_to_rows_gap < wide.sizing.header_to_rows_gap);
        assert!(narrow.sizing.row_corner_inset < wide.sizing.row_corner_inset);
        assert!(narrow.sizing.header_label_gutter < wide.sizing.header_label_gutter);
        assert!(narrow.sizing.status_segment_gap < wide.sizing.status_segment_gap);
        assert!(narrow.sizing.recovery_badge_height < wide.sizing.recovery_badge_height);
        assert!(narrow.sizing.recovery_badge_min_width < wide.sizing.recovery_badge_min_width);
        assert!(narrow.theme.state_hover_strong < wide.theme.state_hover_strong);
        assert!(narrow.theme.motion_speed_transport < wide.theme.motion_speed_transport);
        assert!(narrow.theme.motion_focus_wave_amp < wide.theme.motion_focus_wave_amp);
    }

    #[test]
    fn shell_specific_sidebar_chrome_stays_outside_core_theme() {
        let tokens = StyleTokens::for_viewport_width(1280.0);
        assert_eq!(
            tokens.chrome.source_recovery_badge_active,
            tokens.theme.accent_warning
        );
        assert_ne!(tokens.chrome.source_section_divider, tokens.theme.border);
    }
}
