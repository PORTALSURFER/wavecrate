#[cfg(test)]
use super::ShellLayout;
#[cfg(test)]
use crate::gui::native_shell::style::StyleTokens;

/// Derived metrics used to validate layout parity contracts.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(test)]
pub(crate) struct LayoutContractSnapshot {
    /// Effective viewport width after layout clamping.
    pub viewport_width: f32,
    /// Effective viewport height after layout clamping.
    pub viewport_height: f32,
    /// Sidebar width in logical pixels.
    pub sidebar_width: f32,
    /// Waveform card height in logical pixels.
    pub waveform_height: f32,
    /// Browser table row capacity using active row-height tokens.
    pub browser_row_capacity: usize,
    /// Top-bar height in logical pixels.
    pub top_bar_height: f32,
    /// Status-bar height in logical pixels.
    pub status_bar_height: f32,
}

/// Build the compact layout metric snapshot used by contract tests.
#[cfg(test)]
pub(super) fn snapshot(layout: &ShellLayout, style: &StyleTokens) -> LayoutContractSnapshot {
    let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
    LayoutContractSnapshot {
        viewport_width: layout.root.rect.width(),
        viewport_height: layout.root.rect.height(),
        sidebar_width: layout.sidebar.width(),
        waveform_height: layout.waveform_card.height(),
        browser_row_capacity: (layout.browser_rows.height() / row_stride).floor() as usize,
        top_bar_height: layout.top_bar.height(),
        status_bar_height: layout.status_bar.height(),
    }
}
