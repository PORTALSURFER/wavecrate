//! Shared style tokens for the native shell renderer.

use crate::gui::types::Rgba8;

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

/// Style tokens consumed by the retained shell paint pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StyleTokens {
    /// Root clear color for the frame.
    pub clear_color: Rgba8,
    /// Primary surface fill.
    pub bg_primary: Rgba8,
    /// Secondary surface fill.
    pub bg_secondary: Rgba8,
    /// Tertiary/raised surface fill.
    pub bg_tertiary: Rgba8,
    /// Standard border color.
    pub border: Rgba8,
    /// Primary grid line color.
    pub grid_strong: Rgba8,
    /// Secondary grid line color.
    pub grid_soft: Rgba8,
    /// Primary selection accent.
    pub accent_mint: Rgba8,
    /// Secondary accent.
    pub accent_copper: Rgba8,
    /// Warning/hover accent.
    pub accent_warning: Rgba8,
    /// High-contrast text color.
    pub text_primary: Rgba8,
    /// Secondary muted text color.
    pub text_muted: Rgba8,
}

impl Default for StyleTokens {
    fn default() -> Self {
        Self {
            clear_color: rgba(12, 11, 10, 255),
            bg_primary: rgba(12, 11, 10, 255),
            bg_secondary: rgba(20, 18, 16, 255),
            bg_tertiary: rgba(28, 26, 23, 255),
            border: rgba(44, 40, 36, 255),
            grid_strong: rgba(55, 50, 45, 255),
            grid_soft: rgba(42, 38, 34, 255),
            accent_mint: rgba(152, 172, 158, 255),
            accent_copper: rgba(186, 148, 108, 255),
            accent_warning: rgba(194, 158, 108, 255),
            text_primary: rgba(224, 227, 234, 255),
            text_muted: rgba(166, 173, 184, 255),
        }
    }
}
