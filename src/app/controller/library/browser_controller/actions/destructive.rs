use super::*;
use std::collections::HashSet;
use std::path::PathBuf;

impl BrowserController<'_> {
    pub(super) fn delete_browser_sample_action(&mut self, row: usize) -> Result<(), String> {
        self.delete_browser_samples_action(&[row])
    }

    pub(super) fn delete_browser_samples_action(&mut self, rows: &[usize]) -> Result<(), String> {
        let next_focus = self.next_browser_focus_after_delete(rows);
        let (contexts, initial_error) = self.resolve_unique_browser_contexts(rows);
        self.delete_browser_contexts_action(next_focus, contexts, initial_error)
    }

    pub(crate) fn delete_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
        primary_visible_row: Option<usize>,
    ) -> Result<(), String> {
        let next_focus = if let Some(row) = primary_visible_row {
            let action_rows = self.action_rows_from_primary(row);
            self.next_browser_focus_after_delete(&action_rows)
        } else {
            None
        };
        let (contexts, initial_error) = self.resolve_unique_browser_contexts_for_paths(paths);
        self.delete_browser_contexts_action(next_focus, contexts, initial_error)
    }

    fn delete_browser_contexts_action(
        &mut self,
        next_focus: Option<super::super::helpers::DeleteBrowserFocusPlan>,
        contexts: Vec<super::super::helpers::TriageSampleContext>,
        initial_error: Option<String>,
    ) -> Result<(), String> {
        if self.warn_if_any_browser_context_busy(&contexts, "deleting") {
            return Ok(());
        }
        if let Some(message) = self.loading_delete_block_message(&contexts) {
            self.set_status(message, StatusTone::Info);
            return Ok(());
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return Ok(());
        }
        let focus_path = next_focus
            .as_ref()
            .and_then(|plan| plan.preferred_path.clone());
        let samples = contexts
            .into_iter()
            .map(|ctx| (ctx.source, ctx.entry))
            .collect::<Vec<_>>();
        let selected_source_id = self.selected_source_id();
        let similar_query = self.ui.browser.search.similar_query.clone();
        let moved = self.move_samples_to_configured_trash_detailed(samples, focus_path);
        if !moved.moved_paths.is_empty() {
            let deleted_paths = moved.moved_paths.iter().cloned().collect::<HashSet<_>>();
            crate::app::controller::library::wavs::schedule_similarity_filter_rebuild_after_delete_with_state(
                self,
                selected_source_id,
                similar_query,
                &deleted_paths,
            );
            crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
        }
        self.finish_delete_browser_samples(moved, initial_error)?;
        Ok(())
    }

    fn loading_delete_block_message(
        &self,
        contexts: &[super::super::helpers::TriageSampleContext],
    ) -> Option<String> {
        let pending_audio = self.runtime.jobs.pending_audio();
        let blocked = contexts.iter().find(|ctx| {
            pending_audio.as_ref().is_some_and(|pending| {
                pending.source_id == ctx.source.id
                    && pending.relative_path == ctx.entry.relative_path
            }) || self.ui.waveform.loading.as_deref() == Some(ctx.entry.relative_path.as_path())
        })?;
        Some(format!(
            "Wait for sample load to finish before deleting {}",
            blocked.entry.relative_path.display()
        ))
    }

    fn finish_delete_browser_samples(
        &mut self,
        moved: crate::app::controller::library::trash::ConfiguredTrashMoveResult,
        initial_error: Option<String>,
    ) -> Result<(), String> {
        let error_count = moved.errors.len() + usize::from(initial_error.is_some());
        let moved_count = moved.moved_count();
        if error_count == 0 {
            if moved_count > 0 {
                self.set_status(
                    format!(
                        "Moved {} {} to trash",
                        moved_count,
                        sample_label(moved_count)
                    ),
                    StatusTone::Info,
                );
            }
            return Ok(());
        }
        let last_error = moved
            .errors
            .last()
            .cloned()
            .or(initial_error)
            .unwrap_or_default();
        if last_error.is_empty() {
            return Ok(());
        }
        let message = if moved_count == 0 {
            last_error
        } else {
            format!(
                "Moved {} {} to trash with {} {}: {}",
                moved_count,
                sample_label(moved_count),
                error_count,
                error_label(error_count),
                last_error
            )
        };
        let tone = if moved_count == 0 {
            StatusTone::Error
        } else {
            StatusTone::Warning
        };
        self.set_status(message.clone(), tone);
        Err(message)
    }

    pub(crate) fn restore_browser_focus_after_delete(
        &mut self,
        next_focus: Option<super::super::helpers::DeleteBrowserFocusPlan>,
    ) {
        let Some(next_focus) = next_focus else {
            return;
        };
        if let Some(path) = next_focus.preferred_path.as_ref()
            && let Some(row) = self.visible_row_for_path(path)
        {
            self.focus_browser_row_only(row);
            return;
        }
        let Some(fallback_visible_row) = next_focus.fallback_visible_row else {
            return;
        };
        let visible_len = self.ui.browser.viewport.visible.len();
        if visible_len == 0 {
            return;
        }
        self.focus_browser_row_only(fallback_visible_row.min(visible_len.saturating_sub(1)));
    }
}

fn sample_label(count: usize) -> &'static str {
    if count == 1 { "sample" } else { "samples" }
}

fn error_label(count: usize) -> &'static str {
    if count == 1 { "error" } else { "errors" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::app::state::{SampleBrowserSort, SimilarQuery};
    use crate::sample_sources::Rating;
    use std::path::Path;

    #[test]
    fn restore_browser_focus_after_delete_uses_visible_fallback_when_preferred_path_is_hidden() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("a.wav", Rating::NEUTRAL),
            sample_entry("b.wav", Rating::NEUTRAL),
            sample_entry("c.wav", Rating::NEUTRAL),
        ]);
        controller.focus_browser_row_only(0);
        let next_focus = {
            let mut browser = BrowserController::new(&mut controller);
            browser.next_browser_focus_after_delete(&[0])
        };
        controller.runtime.jobs.pending_audio = None;
        controller.runtime.jobs.pending_playback = None;
        controller.ui.browser.search.similar_query = Some(SimilarQuery {
            sample_id: "source::c.wav".to_string(),
            label: "c.wav".to_string(),
            indices: vec![2],
            scores: vec![1.0],
            anchor_index: Some(2),
        });
        controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
        controller.rebuild_browser_lists();

        {
            let mut browser = BrowserController::new(&mut controller);
            browser.restore_browser_focus_after_delete(next_focus);
        }

        assert_eq!(
            controller.focused_browser_path().as_deref(),
            Some(Path::new("c.wav"))
        );
        assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
        assert!(controller.runtime.jobs.pending_audio.is_none());
        assert!(controller.runtime.jobs.pending_playback.is_none());
        assert!(controller.ui.browser.selection.commit_focus_pending);
    }
}
