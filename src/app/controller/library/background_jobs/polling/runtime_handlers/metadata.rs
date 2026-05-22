use super::*;
use crate::app::controller::jobs::MetadataMutationResult;
use tracing::warn;

impl AppController {
    /// Apply one completed metadata mutation batch to optimistic controller state.
    pub(crate) fn handle_metadata_mutation_finished_message(
        &mut self,
        message: MetadataMutationResult,
    ) {
        let Some(pending) = self
            .runtime
            .source_lane
            .mutations
            .finish_metadata_mutation(message.request_id)
        else {
            return;
        };
        self.extend_selected_source_mutation_claim_grace(&pending.source_id);
        if let Err(err) = message.result {
            self.rollback_metadata_mutation(&pending.source_id, &pending.rollback);
            if !pending.blocks_file_mutation && is_busy_lock_error_message(&err) {
                warn!(
                    source_id = %pending.source_id,
                    request_id = message.request_id,
                    elapsed_ms = message.elapsed.as_millis(),
                    error = %err,
                    "background analysis metadata mutation hit busy lock"
                );
                return;
            }
            self.set_status(format!("Metadata update failed: {err}"), StatusTone::Error);
            return;
        }
        self.finish_metadata_mutation_intents(&pending.source_id, &pending.rollback);
        if pending.refresh_browser_projection
            && self.selection_state.ctx.selected_source.as_ref() == Some(&pending.source_id)
        {
            let source_revision = self
                .current_source()
                .filter(|source| source.id == pending.source_id)
                .and_then(|source| self.database_for(&source).ok())
                .and_then(|db| db.get_revision().ok());
            self.ui_cache
                .browser
                .pipeline
                .sync_source_revision(source_revision);
            self.mark_browser_search_projection_revision_dirty();
            let metadata_delta_paths = pending.paths.iter().cloned().collect::<Vec<_>>();
            if self.should_dispatch_browser_search_async() {
                self.dispatch_search_job_with_metadata_delta(metadata_delta_paths);
            } else {
                self.rebuild_browser_lists_with_metadata_delta(metadata_delta_paths);
            }
        }
    }
}

fn is_busy_lock_error_message(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}
