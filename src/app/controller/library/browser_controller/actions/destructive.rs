use super::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

struct DeleteAttemptSummary {
    deleted_paths: HashSet<PathBuf>,
    deleted_count: usize,
    error_count: usize,
    last_error: Option<String>,
}

impl DeleteAttemptSummary {
    fn new(initial_error: Option<String>) -> Self {
        Self {
            deleted_paths: HashSet::new(),
            deleted_count: 0,
            error_count: usize::from(initial_error.is_some()),
            last_error: initial_error,
        }
    }

    fn record_deleted_path(&mut self, path: &Path) {
        self.deleted_paths.insert(path.to_path_buf());
        self.deleted_count += 1;
    }

    fn record_error(&mut self, err: String) {
        self.error_count += 1;
        self.last_error = Some(err);
    }
}

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
        let selected_source_id = self.selected_source_id();
        let similar_query = self.ui.browser.search.similar_query.clone();
        if self.warn_if_any_browser_context_busy(&contexts, "deleting") {
            return Ok(());
        }
        if let Some(message) = self.loading_delete_block_message(&contexts) {
            self.set_status(message, StatusTone::Info);
            return Ok(());
        }
        let summary = self.delete_browser_contexts(contexts, initial_error);
        if !summary.deleted_paths.is_empty() {
            crate::app::controller::library::wavs::schedule_similarity_filter_rebuild_after_delete_with_state(
                self,
                selected_source_id,
                similar_query,
                &summary.deleted_paths,
            );
            crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
            self.restore_browser_focus_after_delete(next_focus);
        }
        self.finish_delete_browser_samples(summary)
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

    fn delete_browser_contexts(
        &mut self,
        contexts: Vec<super::super::helpers::TriageSampleContext>,
        initial_error: Option<String>,
    ) -> DeleteAttemptSummary {
        let mut summary = DeleteAttemptSummary::new(initial_error);
        for ctx in contexts {
            match self.try_delete_browser_sample_ctx(&ctx) {
                Ok(()) => summary.record_deleted_path(&ctx.entry.relative_path),
                Err(err) => summary.record_error(err),
            }
        }
        summary
    }

    fn finish_delete_browser_samples(
        &mut self,
        summary: DeleteAttemptSummary,
    ) -> Result<(), String> {
        if summary.error_count == 0 {
            return Ok(());
        }
        let Some(last_error) = summary.last_error else {
            return Ok(());
        };
        let message = if summary.deleted_count == 0 {
            last_error
        } else {
            format!(
                "Deleted {} {} with {} {}: {}",
                summary.deleted_count,
                sample_label(summary.deleted_count),
                summary.error_count,
                error_label(summary.error_count),
                last_error
            )
        };
        let tone = if summary.deleted_count == 0 {
            StatusTone::Error
        } else {
            StatusTone::Warning
        };
        self.set_status(message.clone(), tone);
        Err(message)
    }

    fn restore_browser_focus_after_delete(
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
