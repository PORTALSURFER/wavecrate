//! Reverse-lookup caches for browser visible-row and triage-column projections.

use super::*;

impl AppController {
    /// Invalidate retained browser reverse lookups after visible rows change.
    pub(crate) fn invalidate_browser_lookup_maps(&mut self) {
        let stale_revision = self
            .ui
            .browser
            .viewport
            .visible_rows_revision
            .wrapping_sub(1);
        self.ui.browser.viewport.visible_row_lookup_revision = stale_revision;
        self.ui.browser.viewport.triage_index_lookup_revision = stale_revision;
    }

    /// Ensure the visible-row reverse lookup matches the current browser projection revision.
    fn ensure_browser_visible_row_lookup_current(&mut self) {
        let entries_len = self.wav_entries_len();
        if self.ui.browser.viewport.visible_row_lookup_revision
            == self.ui.browser.viewport.visible_rows_revision
            && self.ui.browser.viewport.visible_row_by_absolute.len() >= entries_len
            && self
                .ui
                .browser
                .viewport
                .visible_row_by_absolute_generation
                .len()
                >= entries_len
        {
            return;
        }
        self.rebuild_browser_visible_row_lookup();
    }

    /// Ensure the triage-column reverse lookup matches the current browser projection revision.
    fn ensure_browser_triage_lookup_current(&mut self) {
        let entries_len = self.wav_entries_len();
        if self.ui.browser.viewport.triage_index_lookup_revision
            == self.ui.browser.viewport.visible_rows_revision
            && self.ui.browser.viewport.triage_index_by_absolute.len() >= entries_len
            && self
                .ui
                .browser
                .viewport
                .triage_index_by_absolute_generation
                .len()
                >= entries_len
        {
            return;
        }
        self.rebuild_browser_triage_lookup();
    }

    /// Grow retained visible-row lookup storage to cover the current entry count.
    fn ensure_browser_visible_row_lookup_capacity(&mut self, entries_len: usize) {
        if self.ui.browser.viewport.visible_row_by_absolute.len() < entries_len {
            self.ui
                .browser
                .viewport
                .visible_row_by_absolute
                .resize(entries_len, None);
        }
        if self
            .ui
            .browser
            .viewport
            .visible_row_by_absolute_generation
            .len()
            < entries_len
        {
            self.ui
                .browser
                .viewport
                .visible_row_by_absolute_generation
                .resize(entries_len, 0);
        }
    }

    /// Grow retained triage-column lookup storage to cover the current entry count.
    fn ensure_browser_triage_lookup_capacity(&mut self, entries_len: usize) {
        if self.ui.browser.viewport.triage_index_by_absolute.len() < entries_len {
            self.ui
                .browser
                .viewport
                .triage_index_by_absolute
                .resize(entries_len, None);
        }
        if self
            .ui
            .browser
            .viewport
            .triage_index_by_absolute_generation
            .len()
            < entries_len
        {
            self.ui
                .browser
                .viewport
                .triage_index_by_absolute_generation
                .resize(entries_len, 0);
        }
    }

    /// Rebuild the visible-row reverse lookup for the current browser projection.
    fn rebuild_browser_visible_row_lookup(&mut self) {
        let entries_len = self.wav_entries_len();
        let lookup_revision = self.ui.browser.viewport.visible_rows_revision;
        self.ensure_browser_visible_row_lookup_capacity(entries_len);
        match &self.ui.browser.viewport.visible {
            crate::app::state::VisibleRows::All { total } => {
                let limit = (*total).min(entries_len);
                for index in 0..limit {
                    self.ui.browser.viewport.visible_row_by_absolute[index] = Some(index);
                    self.ui.browser.viewport.visible_row_by_absolute_generation[index] =
                        lookup_revision;
                }
            }
            crate::app::state::VisibleRows::List(rows) => {
                for (row, index) in rows.iter().copied().enumerate() {
                    if index < entries_len {
                        self.ui.browser.viewport.visible_row_by_absolute[index] = Some(row);
                        self.ui.browser.viewport.visible_row_by_absolute_generation[index] =
                            lookup_revision;
                    }
                }
            }
        }
        self.ui.browser.viewport.visible_row_lookup_revision = lookup_revision;
    }

    /// Rebuild the triage-column reverse lookup for the current browser projection.
    fn rebuild_browser_triage_lookup(&mut self) {
        let entries_len = self.wav_entries_len();
        let lookup_revision = self.ui.browser.viewport.visible_rows_revision;
        self.ensure_browser_triage_lookup_capacity(entries_len);
        for (row, index) in self.ui.browser.trash.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.viewport.triage_index_by_absolute[index] =
                    Some(SampleBrowserIndex {
                        column: crate::app::state::TriageFlagColumn::Trash,
                        row,
                    });
                self.ui.browser.viewport.triage_index_by_absolute_generation[index] =
                    lookup_revision;
            }
        }
        for (row, index) in self.ui.browser.neutral.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.viewport.triage_index_by_absolute[index] =
                    Some(SampleBrowserIndex {
                        column: crate::app::state::TriageFlagColumn::Neutral,
                        row,
                    });
                self.ui.browser.viewport.triage_index_by_absolute_generation[index] =
                    lookup_revision;
            }
        }
        for (row, index) in self.ui.browser.keep.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.viewport.triage_index_by_absolute[index] =
                    Some(SampleBrowserIndex {
                        column: crate::app::state::TriageFlagColumn::Keep,
                        row,
                    });
                self.ui.browser.viewport.triage_index_by_absolute_generation[index] =
                    lookup_revision;
            }
        }
        self.ui.browser.viewport.triage_index_lookup_revision = lookup_revision;
    }

    /// Resolve the visible-row index for an absolute wav-entry index.
    pub(crate) fn browser_visible_row_for_entry(&mut self, entry_index: usize) -> Option<usize> {
        if entry_index >= self.wav_entries_len() {
            return None;
        }
        self.ensure_browser_visible_row_lookup_current();
        if self
            .ui
            .browser
            .viewport
            .visible_row_by_absolute_generation
            .get(entry_index)
            .copied()
            != Some(self.ui.browser.viewport.visible_rows_revision)
        {
            return None;
        }
        self.ui
            .browser
            .viewport
            .visible_row_by_absolute
            .get(entry_index)
            .copied()
            .flatten()
    }

    /// Resolve a triage-column browser index for an absolute wav entry index.
    pub(super) fn browser_index_for_entry(
        &mut self,
        entry_index: usize,
    ) -> Option<SampleBrowserIndex> {
        if entry_index >= self.wav_entries_len() {
            return None;
        }
        self.ensure_browser_triage_lookup_current();
        if self
            .ui
            .browser
            .viewport
            .triage_index_by_absolute_generation
            .get(entry_index)
            .copied()
            != Some(self.ui.browser.viewport.visible_rows_revision)
        {
            return None;
        }
        self.ui
            .browser
            .viewport
            .triage_index_by_absolute
            .get(entry_index)
            .copied()
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::sample_sources::Rating;

    #[test]
    fn browser_visible_lookup_rebuilds_lazily_and_keeps_triage_stale() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::TRASH_1),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        let stale_revision = controller
            .ui
            .browser
            .viewport
            .visible_rows_revision
            .wrapping_sub(1);
        assert_eq!(
            controller.ui.browser.viewport.visible_row_lookup_revision,
            stale_revision
        );
        assert_eq!(
            controller.ui.browser.viewport.triage_index_lookup_revision,
            stale_revision
        );

        assert_eq!(controller.browser_visible_row_for_entry(1), Some(1));
        assert_eq!(
            controller.ui.browser.viewport.visible_row_lookup_revision,
            controller.ui.browser.viewport.visible_rows_revision
        );
        assert_eq!(
            controller.ui.browser.viewport.triage_index_lookup_revision,
            stale_revision
        );
    }

    #[test]
    fn browser_triage_lookup_rebuilds_lazily_and_keeps_visible_stale() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::TRASH_1),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        let stale_revision = controller
            .ui
            .browser
            .viewport
            .visible_rows_revision
            .wrapping_sub(1);

        assert_eq!(
            controller.browser_index_for_entry(0),
            Some(SampleBrowserIndex {
                column: crate::app::state::TriageFlagColumn::Trash,
                row: 0,
            })
        );
        assert_eq!(
            controller.ui.browser.viewport.triage_index_lookup_revision,
            controller.ui.browser.viewport.visible_rows_revision
        );
        assert_eq!(
            controller.ui.browser.viewport.visible_row_lookup_revision,
            stale_revision
        );
    }
}
