//! Semantic theme + compatibility palette values for the native shell.

use crate::theme::ThemeTokens;

use super::LayoutScaleTier;
use super::chrome::ShellChromeTokens;

/// Resolve the shell theme and compatibility chrome for the requested layout tier.
pub(super) fn tokens_for_tier(layout_tier: LayoutScaleTier) -> (ThemeTokens, ShellChromeTokens) {
    let mut theme = sempal_dark_theme();
    let mut chrome = ShellChromeTokens {
        source_section_divider: rgba(46, 54, 59, 255),
        source_recovery_badge_idle: rgba(32, 38, 42, 255),
        source_recovery_badge_active: theme.accent_warning,
    };

    match layout_tier {
        LayoutScaleTier::Compact => {
            theme.surface_overlay = rgba(25, 28, 29, 255);
            theme.border_emphasis = rgba(52, 56, 57, 255);
            theme.control_disabled_fill = rgba(20, 23, 24, 255);
            chrome.source_section_divider = rgba(41, 45, 46, 255);
            chrome.source_recovery_badge_idle = rgba(28, 31, 32, 255);
            chrome.source_recovery_badge_active = theme.accent_warning;
        }
        LayoutScaleTier::Wide => {
            theme.surface_overlay = rgba(33, 36, 37, 255);
            theme.border_emphasis = rgba(67, 71, 72, 255);
            theme.control_disabled_fill = rgba(26, 29, 30, 255);
            chrome.source_section_divider = rgba(50, 54, 55, 255);
            chrome.source_recovery_badge_idle = rgba(36, 39, 40, 255);
            chrome.source_recovery_badge_active = theme.accent_warning;
        }
        LayoutScaleTier::Standard => {}
    }

    (theme, chrome)
}

fn sempal_dark_theme() -> ThemeTokens {
    let mut theme = ThemeTokens::default();
    theme.clear_color = rgba(15, 17, 18, 255);
    theme.bg_primary = rgba(15, 17, 18, 255);
    theme.bg_secondary = rgba(18, 20, 21, 255);
    theme.bg_tertiary = rgba(24, 26, 27, 255);
    theme.surface_base = rgba(20, 22, 23, 255);
    theme.surface_raised = rgba(25, 27, 28, 255);
    theme.surface_overlay = rgba(29, 32, 33, 255);
    theme.border = rgba(41, 44, 45, 255);
    theme.border_emphasis = rgba(59, 63, 64, 255);
    theme.grid_strong = rgba(48, 51, 52, 255);
    theme.grid_soft = rgba(32, 35, 36, 255);
    theme.accent_mint = rgba(232, 149, 120, 255);
    theme.accent_copper = rgba(199, 121, 97, 255);
    theme.accent_danger = rgba(212, 107, 94, 255);
    theme.accent_warning = rgba(240, 160, 127, 255);
    theme.highlight_orange = rgba(242, 154, 122, 255);
    theme.highlight_orange_soft = rgba(189, 126, 104, 255);
    theme.highlight_blue = rgba(167, 104, 88, 255);
    theme.highlight_blue_soft = rgba(212, 138, 114, 255);
    theme.highlight_cyan = rgba(210, 138, 112, 255);
    theme.highlight_cyan_soft = rgba(188, 128, 105, 255);
    theme.text_primary = rgba(215, 215, 210, 255);
    theme.text_muted = rgba(139, 146, 149, 255);
    theme.control_disabled_fill = rgba(22, 27, 31, 255);
    theme
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> crate::gui::types::Rgba8 {
    crate::gui::types::Rgba8 { r, g, b, a }
}
