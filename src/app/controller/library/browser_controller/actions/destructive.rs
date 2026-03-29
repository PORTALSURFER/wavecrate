use super::*;
use std::collections::HashSet;

impl BrowserController<'_> {
    pub(super) fn delete_browser_sample_action(&mut self, row: usize) -> Result<(), String> {
        self.delete_browser_samples_action(&[row])
    }

    pub(super) fn delete_browser_samples_action(&mut self, rows: &[usize]) -> Result<(), String> {
        let next_focus = self.next_browser_focus_after_delete(rows);
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
        if self.warn_if_any_browser_context_busy(&contexts, "deleting") {
            return Ok(());
        }
        let deleted_paths: HashSet<_> = contexts
            .iter()
            .map(|ctx| ctx.entry.relative_path.clone())
            .collect();
        crate::app::controller::library::wavs::schedule_similarity_filter_rebuild_after_delete(
            self,
            &deleted_paths,
        );
        for ctx in contexts {
            if let Err(err) = self.try_delete_browser_sample_ctx(&ctx) {
                last_error = Some(err);
            }
        }
        self.restore_browser_focus_after_delete(next_focus);
        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn restore_browser_focus_after_delete(&mut self, next_focus: Option<std::path::PathBuf>) {
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
