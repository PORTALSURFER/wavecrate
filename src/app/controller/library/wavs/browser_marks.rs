//! Session-scoped browser sample marking helpers.
//!
//! Marks let users flag samples for short-lived review without changing source
//! metadata. The state lives only in the controller/browser UI model and is
//! keyed by source-relative path so it survives source switches within one run.

use super::*;
use std::path::{Path, PathBuf};

impl AppController {
    /// Return whether one active-source sample path is marked in the current session.
    pub(crate) fn browser_sample_marked(&self, source_id: &SourceId, path: &Path) -> bool {
        self.ui.browser.marks.contains(source_id, path)
    }

    /// Toggle the session mark for the focused row or current browser multi-selection.
    pub(crate) fn toggle_browser_sample_mark_action(&mut self) {
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            return;
        };
        let Some(target_paths) = self.browser_mark_target_paths() else {
            return;
        };
        let fallback_path = self.browser_mark_fallback_path(&target_paths);
        if !self.ui.browser.marks.toggle_paths(&source_id, &target_paths) {
            return;
        }
        if let Some(path) = fallback_path {
            self.focus_wav_by_path_preview_with_rebuild(&path, false);
        }
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
        if self.should_dispatch_browser_search_async() {
            self.dispatch_search_job();
        } else {
            self.rebuild_browser_lists();
        }
    }

    /// Toggle the browser marked-only filter and refresh visible rows.
    pub(crate) fn toggle_browser_marked_filter_action(&mut self) {
        browser_search::toggle_browser_marked_filter(self);
    }

    /// Remap a marked path after an in-source rename or move.
    pub(crate) fn remap_browser_marked_path(
        &mut self,
        source_id: &SourceId,
        old_path: &Path,
        new_path: &Path,
    ) {
        if !self.ui.browser.marks.remap_path(source_id, old_path, new_path) {
            return;
        }
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
    }

    /// Remove a marked path after the sample disappears from a source.
    pub(crate) fn clear_browser_marked_path(&mut self, source_id: &SourceId, relative_path: &Path) {
        if !self.ui.browser.marks.remove_path(source_id, relative_path) {
            return;
        }
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
    }

    /// Drop stale marks for the selected source when entries are rebuilt or mutated.
    pub(crate) fn prune_browser_marks_for_selected_source(&mut self) {
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            return;
        };
        let retained: std::collections::HashSet<PathBuf> = (0..self.wav_entries_len())
            .filter_map(|index| self.wav_entry(index).map(|entry| entry.relative_path.clone()))
            .collect();
        let changed = self.ui.browser.marks.retain_paths_for_source(&source_id, |path| {
            retained.contains(path)
        });
        if !changed {
            return;
        }
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
    }

    fn browser_mark_target_paths(&mut self) -> Option<Vec<PathBuf>> {
        let primary_row = self.focused_browser_row()?;
        let rows = self.action_rows_from_primary(primary_row);
        let mut paths = Vec::with_capacity(rows.len());
        for row in rows {
            let path = self.browser_path_for_visible(row)?;
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        (!paths.is_empty()).then_some(paths)
    }

    fn browser_mark_fallback_path(&mut self, target_paths: &[PathBuf]) -> Option<PathBuf> {
        if !self.ui.browser.search.marked_only {
            return None;
        }
        let focused_row = self.focused_browser_row()?;
        let focused_path = self.browser_path_for_visible(focused_row)?;
        if !target_paths.iter().any(|path| path == &focused_path) {
            return None;
        }
        let visible_len = self.ui.browser.viewport.visible.len();
        ((focused_row + 1)..visible_len)
            .chain(0..focused_row)
            .find_map(|row| {
                self.browser_path_for_visible(row)
                    .filter(|path| !target_paths.iter().any(|target| target == path))
            })
    }
}
