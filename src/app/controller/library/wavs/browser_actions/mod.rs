//! Browser action orchestration for focus navigation, selection invariants, and row actions.
//!
//! This module keeps the public `AppController` browser-action surface stable while splitting
//! the implementation by responsibility: focus/navigation, multi-selection state management,
//! and prompt/row actions.

use super::*;

mod focus_navigation;
mod row_actions;
mod selection;

#[cfg(test)]
mod tests;

/// Internal selection intents shared across browser-action helpers.
#[derive(Clone, Copy)]
pub(super) enum SelectionAction {
    Replace,
    Toggle,
    Extend { additive: bool },
}

/// Intent for browser-row focus updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserFocusIntent {
    /// Preview navigation without committing expensive side effects.
    Preview,
    /// Full commit navigation with selection-loading side effects.
    Commit,
}

impl AppController {
    /// Resolve the visible browser row reached by moving `delta` from the current focus.
    pub(super) fn browser_target_visible_row_from_delta(&self, delta: i8) -> Option<usize> {
        let visible_count = self.ui.browser.visible.len();
        if visible_count == 0 {
            return None;
        }
        let base = self
            .ui
            .browser
            .selected_visible
            .unwrap_or(0)
            .min(visible_count - 1);
        Some((base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize)
    }
}
