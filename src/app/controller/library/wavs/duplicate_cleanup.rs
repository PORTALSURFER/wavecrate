//! Browser duplicate-cleanup controller actions.

use super::*;
use crate::app::state::{BrowserDuplicateCleanupState, SampleBrowserTab};
use crate::app::view_model;
use std::path::PathBuf;

impl AppController {
    /// Return whether browser duplicate cleanup is currently active.
    pub(crate) fn browser_duplicate_cleanup_active(&self) -> bool {
        self.ui.browser.duplicate_cleanup.is_some()
    }

    /// Clear the active browser duplicate-cleanup workspace without changing other filters.
    pub(crate) fn clear_browser_duplicate_cleanup(&mut self) -> bool {
        if self.ui.browser.duplicate_cleanup.take().is_none() {
            return false;
        }
        self.rebuild_browser_lists();
        true
    }

    /// Clear duplicate cleanup when its source or anchor no longer matches live browser state.
    pub(crate) fn clear_invalid_browser_duplicate_cleanup(&mut self) {
        let cleanup = self.ui.browser.duplicate_cleanup.clone();
        let should_clear = cleanup.as_ref().is_some_and(|cleanup| {
            self.selection_state.ctx.selected_source.as_ref() != Some(&cleanup.source_id)
                || self.wav_index_for_path(&cleanup.anchor_path).is_none()
        });
        if should_clear {
            self.ui.browser.duplicate_cleanup = None;
        }
    }

    /// Toggle browser duplicate-cleanup mode for the currently focused sample.
    pub(crate) fn toggle_browser_duplicate_cleanup_mode(&mut self) {
        if self.clear_browser_duplicate_cleanup() {
            self.set_status("Duplicate cleanup off", StatusTone::Info);
            return;
        }
        match self.enter_browser_duplicate_cleanup_mode() {
            Ok(()) => {}
            Err(err) => self.set_status(err, StatusTone::Warning),
        }
    }

    /// Enter duplicate cleanup mode using the currently focused browser sample as the anchor.
    pub(crate) fn enter_browser_duplicate_cleanup_mode(&mut self) -> Result<(), String> {
        let Some(visible_row) = self.focused_browser_row() else {
            return Err("Focus a sample to clean duplicates".to_string());
        };
        let source_id = self
            .selection_state
            .ctx
            .selected_source
            .clone()
            .ok_or_else(|| "Select a source before cleaning duplicates".to_string())?;
        let sample_id = self.sample_id_for_visible_row(visible_row)?;
        let anchor_index = self
            .ui
            .browser
            .viewport
            .visible
            .get(visible_row)
            .ok_or_else(|| "Focused sample is out of range".to_string())?;
        let anchor_path = self
            .wav_entry(anchor_index)
            .map(|entry| entry.relative_path.clone())
            .ok_or_else(|| "Focused sample entry missing".to_string())?;
        let query = super::similar::build_duplicate_query_for_sample_id(
            self,
            &sample_id,
            Some(anchor_index),
        )?;
        let cleanup = BrowserDuplicateCleanupState::new(
            source_id,
            sample_id,
            anchor_path.clone(),
            query.label,
            query.indices,
            query.scores,
            query.anchor_index.unwrap_or(anchor_index),
        );
        self.ui.browser.active_tab = SampleBrowserTab::List;
        self.clear_similar_filter();
        self.ui.browser.duplicate_cleanup = Some(cleanup);
        self.rebuild_browser_lists();
        if let Some(row) = self.visible_row_for_path(&anchor_path) {
            self.focus_browser_row_only(row);
        }
        let counts = self.browser_duplicate_cleanup_counts().unwrap_or((0, 0));
        self.set_status(
            format!(
                "Duplicate cleanup on for {}. {} duplicate file(s), {} kept. Right-click files to keep, Enter to trash the rest.",
                view_model::sample_display_label(&anchor_path),
                counts.0,
                counts.1
            ),
            StatusTone::Info,
        );
        Ok(())
    }

    /// Toggle whether one visible duplicate-cleanup row should be kept.
    pub(crate) fn toggle_browser_duplicate_cleanup_keep_for_visible_row(
        &mut self,
        visible_row: usize,
    ) -> Result<bool, String> {
        let entry_index = self
            .ui
            .browser
            .viewport
            .visible
            .get(visible_row)
            .ok_or_else(|| "Selected row is out of range".to_string())?;
        let label = self
            .wav_entry(entry_index)
            .map(|entry| view_model::sample_display_label(&entry.relative_path))
            .unwrap_or_else(|| format!("row {}", visible_row + 1));
        let (kept, is_anchor, duplicate_count, kept_count) = {
            let cleanup = self
                .ui
                .browser
                .duplicate_cleanup
                .as_mut()
                .ok_or_else(|| "Duplicate cleanup is not active".to_string())?;
            let kept = cleanup.toggle_keep(entry_index);
            (
                kept,
                cleanup.is_anchor(entry_index),
                cleanup.indices.len().saturating_sub(1),
                cleanup.kept_indices.len(),
            )
        };
        self.focus_browser_row_only(visible_row);
        if is_anchor {
            self.set_status(
                format!(
                    "Anchor kept: {}. {} duplicate file(s), {} kept.",
                    label, duplicate_count, kept_count
                ),
                StatusTone::Info,
            );
            return Ok(true);
        }
        self.set_status(
            if kept {
                format!(
                    "Keeping {}. {} duplicate file(s), {} kept.",
                    label, duplicate_count, kept_count
                )
            } else {
                format!(
                    "Marked {} for trash. {} duplicate file(s), {} kept.",
                    label, duplicate_count, kept_count
                )
            },
            StatusTone::Info,
        );
        Ok(kept)
    }

    /// Move every unkept duplicate candidate in the active cleanup workspace to trash.
    pub(crate) fn confirm_browser_duplicate_cleanup(&mut self) -> Result<(), String> {
        let cleanup = self
            .ui
            .browser
            .duplicate_cleanup
            .clone()
            .ok_or_else(|| "Duplicate cleanup is not active".to_string())?;
        let Some(source) = self
            .current_source()
            .filter(|source| source.id == cleanup.source_id)
        else {
            return Err("Duplicate cleanup source is no longer selected".to_string());
        };
        let unkept = cleanup.unkept_indices();
        if unkept.is_empty() {
            self.clear_browser_duplicate_cleanup();
            self.set_status(
                "Duplicate cleanup finished: all duplicates were kept",
                StatusTone::Info,
            );
            return Ok(());
        }

        let mut samples = Vec::new();
        for entry_index in unkept {
            let Some(entry) = self.wav_entry(entry_index).cloned() else {
                continue;
            };
            if entry.relative_path == cleanup.anchor_path {
                continue;
            }
            samples.push((source.clone(), entry));
        }
        if samples.is_empty() {
            self.clear_browser_duplicate_cleanup();
            self.set_status(
                "Duplicate cleanup finished: no duplicate files remained to trash",
                StatusTone::Info,
            );
            return Ok(());
        }

        let moved_any =
            self.move_samples_to_configured_trash(samples, Some(cleanup.anchor_path.clone()));
        if !moved_any {
            return Err("Duplicate cleanup could not move any files to trash".to_string());
        }
        self.ui.browser.duplicate_cleanup = None;
        self.rebuild_browser_lists();
        self.refocus_duplicate_cleanup_anchor(&cleanup.anchor_path);
        Ok(())
    }

    fn refocus_duplicate_cleanup_anchor(&mut self, anchor_path: &PathBuf) {
        if let Some(row) = self.visible_row_for_path(anchor_path) {
            self.focus_browser_row_only(row);
        }
    }

    fn browser_duplicate_cleanup_counts(&self) -> Option<(usize, usize)> {
        let cleanup = self.ui.browser.duplicate_cleanup.as_ref()?;
        let duplicate_count = cleanup.indices.len().saturating_sub(1);
        let kept_count = cleanup.kept_indices.len();
        Some((duplicate_count, kept_count))
    }
}
