use super::super::*;
use super::super::trash_move::TrashMoveFinished;
use tracing::warn;

impl AppController {
    pub(crate) fn apply_trash_move_finished(&mut self, result: TrashMoveFinished) {
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        for source_id in &result.affected_sources {
            invalidator.invalidate_all(source_id);
        }

        if let Some(source) = self.current_source()
            && result.affected_sources.iter().any(|id| id == &source.id)
        {
            if let Some(loaded) = self.sample_view.wav.loaded_wav.as_ref() {
                let absolute = source.root.join(loaded);
                if !absolute.is_file() {
                    self.clear_waveform_view();
                }
            }
            self.queue_wav_load();
        }

        if result.cancelled {
            self.set_status(
                format!(
                    "Canceled trash move after {}/{} sample(s)",
                    result.moved, result.total
                ),
                StatusTone::Warning,
            );
        } else if result.total == 0 {
            self.set_status("No trashed samples to move", StatusTone::Info);
        } else if result.errors.is_empty() {
            self.set_status(
                format!("Moved {} trashed sample(s)", result.moved),
                StatusTone::Info,
            );
        } else {
            self.set_status(
                format!(
                    "Moved {} sample(s) with {} error(s)",
                    result.moved,
                    result.errors.len()
                ),
                StatusTone::Warning,
            );
        }

        for err in result.errors {
            warn!(error = %err, moved = result.moved, total = result.total, "Trash move error");
        }
        self.clear_progress();
    }
}
