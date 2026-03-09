use crate::app::controller::StatusTone;
use crate::app::controller::jobs::SourceMoveResult;
use crate::app::controller::ui::drag_drop_controller::DragDropController;
use std::collections::HashSet;
use tracing::info;

impl DragDropController<'_> {
    /// Apply a completed background source move job.
    pub(crate) fn apply_source_move_result(&mut self, result: SourceMoveResult) {
        let Some(target_source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.target_source_id)
            .cloned()
        else {
            self.set_status("Target source not available for move", StatusTone::Error);
            return;
        };
        let moved_sources = self.apply_moved_source_entries(&target_source, &result);
        self.invalidate_moved_sources(&moved_sources);
        self.set_source_move_status(&result);
        for err in &result.errors {
            eprintln!("Source move error: {err}");
        }
        info!(
            "Source move completed: {} moved, {} errors",
            result.moved.len(),
            result.errors.len()
        );
    }

    /// Apply per-sample cache mutations for successful source moves.
    fn apply_moved_source_entries(
        &mut self,
        target_source: &crate::sample_sources::SampleSource,
        result: &SourceMoveResult,
    ) -> HashSet<crate::sample_sources::SourceId> {
        let mut moved_sources = HashSet::new();
        for entry in &result.moved {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == entry.source_id)
                .cloned()
            else {
                continue;
            };
            self.prune_cached_sample(&source, &entry.relative_path);
            self.insert_moved_target_entry(target_source, entry);
            moved_sources.insert(source.id.clone());
            moved_sources.insert(target_source.id.clone());
        }
        moved_sources
    }

    /// Rebuild cached wav state for every source touched by the move job.
    fn invalidate_moved_sources(
        &mut self,
        moved_sources: &HashSet<crate::sample_sources::SourceId>,
    ) {
        for source_id in moved_sources {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == *source_id)
                .cloned()
            else {
                continue;
            };
            self.invalidate_wav_entries_for_source_preserve_folders(&source);
        }
    }

    /// Translate move counts, errors, and cancellation into one user-facing status line.
    fn set_source_move_status(&mut self, result: &SourceMoveResult) {
        let moved = result.moved.len();
        if moved == 0 && result.errors.is_empty() {
            if result.cancelled {
                self.set_status("Move cancelled", StatusTone::Warning);
            } else {
                self.set_status("No samples moved", StatusTone::Warning);
            }
            return;
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = format!("Moved {moved} sample(s)");
        if !result.errors.is_empty() {
            message.push_str(&format!(" with {} error(s)", result.errors.len()));
        }
        if result.cancelled {
            message.push_str(" (cancelled)");
        }
        self.set_status(message, tone);
    }
}
