//! Native-shell theme bridge.

use crate::theme::ThemeTokens;

use super::LayoutScaleTier;
use super::chrome::ShellChromeTokens;

/// Resolve the shell theme and compatibility chrome for the requested layout tier.
pub(super) fn tokens_for_tier(layout_tier: LayoutScaleTier) -> (ThemeTokens, ShellChromeTokens) {
    let theme = ThemeTokens::dark_for_tier(layout_tier);
    let chrome = ShellChromeTokens {
        source_section_divider: theme.border,
        source_recovery_badge_idle: theme.surface_overlay,
        source_recovery_badge_active: theme.accent_warning,
    };

    (theme, chrome)
}
