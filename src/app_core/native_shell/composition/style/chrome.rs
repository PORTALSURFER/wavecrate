//! Compatibility-only chrome tokens for the current Sempal native shell.
//!
//! These values intentionally stay out of the public `radiant::theme` surface
//! because they describe shell-specific sidebar chrome rather than reusable
//! widget/container styling.

use crate::gui::types::Rgba8;

/// Shell-only color tokens that should remain behind the compatibility layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellChromeTokens {
    /// Divider color used between source-management sidebar sections.
    pub source_section_divider: Rgba8,
    /// Recovery badge fill color when entries are present but idle.
    pub source_recovery_badge_idle: Rgba8,
    /// Recovery badge fill color while recovery is actively running.
    pub source_recovery_badge_active: Rgba8,
}
