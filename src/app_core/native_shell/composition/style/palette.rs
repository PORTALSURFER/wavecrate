//! Semantic theme + compatibility palette values for the native shell.

use crate::theme::ThemeTokens;

use super::chrome::ShellChromeTokens;
use super::tier::LayoutScaleTier;

/// Resolve the shell theme and compatibility chrome for the requested layout tier.
pub(super) fn tokens_for_tier(layout_tier: LayoutScaleTier) -> (ThemeTokens, ShellChromeTokens) {
    let mut theme = ThemeTokens::default();
    let mut chrome = ShellChromeTokens {
        source_section_divider: rgba(76, 76, 76, 255),
        source_recovery_badge_idle: rgba(48, 48, 48, 255),
        source_recovery_badge_active: theme.accent_warning,
    };

    match layout_tier {
        LayoutScaleTier::Compact => {
            theme.surface_overlay = rgba(30, 30, 30, 255);
            theme.border_emphasis = rgba(84, 84, 84, 255);
            theme.control_disabled_fill = rgba(34, 34, 34, 255);
            chrome.source_section_divider = rgba(70, 70, 70, 255);
            chrome.source_recovery_badge_idle = rgba(44, 44, 44, 255);
            chrome.source_recovery_badge_active = theme.accent_warning;
        }
        LayoutScaleTier::Wide => {
            theme.surface_overlay = rgba(38, 38, 38, 255);
            theme.border_emphasis = rgba(100, 100, 100, 255);
            theme.control_disabled_fill = rgba(42, 42, 42, 255);
            chrome.source_section_divider = rgba(82, 82, 82, 255);
            chrome.source_recovery_badge_idle = rgba(56, 56, 56, 255);
            chrome.source_recovery_badge_active = theme.accent_warning;
        }
        LayoutScaleTier::Standard => {}
    }

    (theme, chrome)
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> crate::gui::types::Rgba8 {
    crate::gui::types::Rgba8 { r, g, b, a }
}
