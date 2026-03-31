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
        let selected_source_id = self.selected_source_id();
        let similar_query = self.ui.browser.search.similar_query.clone();
        let (contexts, initial_error) = self.resolve_unique_browser_contexts(rows);
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

    fn restore_browser_focus_after_delete(&mut self, next_focus: Option<PathBuf>) {
        let Some(path) = next_focus else {
            return;
        };
        if self.wav_index_for_path(&path).is_none() {
            return;
        }
        if let Some(row) = self.visible_row_for_path(&path) {
            self.focus_browser_row_only(row);
        } else {
            self.select_wav_by_path_with_rebuild(&path, true);
        }
    }
}

fn sample_label(count: usize) -> &'static str {
    if count == 1 { "sample" } else { "samples" }
}

fn error_label(count: usize) -> &'static str {
    if count == 1 { "error" } else { "errors" }
}
