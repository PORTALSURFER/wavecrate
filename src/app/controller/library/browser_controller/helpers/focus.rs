//! Browser deletion focus planning helpers.

use super::*;

impl BrowserController<'_> {
    /// Plan browser focus after deleting the visible rows in `rows`.
    pub(crate) fn next_browser_focus_after_delete(
        &mut self,
        rows: &[usize],
    ) -> Option<DeleteBrowserFocusPlan> {
        if rows.is_empty() || self.ui.browser.viewport.visible.len() == 0 {
            return None;
        }
        let mut sorted = rows.to_vec();
        sorted.sort_unstable();
        let highest = sorted.last().copied()?;
        let first = sorted.first().copied().unwrap_or(highest);
        let after = highest
            .checked_add(1)
            .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
            .and_then(|entry_idx| self.wav_entry(entry_idx))
            .map(|entry| entry.relative_path.clone());
        let fallback_visible_row = if after.is_some() {
            Some(first)
        } else {
            first.checked_sub(1)
        };
        let preferred_path = after.or_else(|| {
            first
                .checked_sub(1)
                .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
                .and_then(|entry_idx| self.wav_entry(entry_idx))
                .map(|entry| entry.relative_path.clone())
        });
        if preferred_path.is_none() && fallback_visible_row.is_none() {
            return None;
        }
        Some(DeleteBrowserFocusPlan {
            preferred_path,
            fallback_visible_row,
        })
    }

    pub(crate) fn warn_if_any_browser_context_busy(
        &mut self,
        contexts: &[TriageSampleContext],
        action: &str,
    ) -> bool {
        let Some(ctx) = contexts.iter().find(|ctx| {
            self.controller
                .runtime
                .recovery
                .active_retained_delete_resolution
                .as_ref()
                .is_some_and(|active| {
                    active.entries.iter().any(|entry| {
                        entry.source_id == ctx.source.id
                            && entry.contains_path(&ctx.entry.relative_path)
                    })
                })
        }) else {
            return false;
        };
        self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            action,
        )
    }
}
