use super::{super::*, DuplicateCleanupCounts, WaveformSliceBatchProfile};

impl AppController {
    /// Keep duplicate cleanup counts synchronized with the current visible preview batch.
    pub(in crate::app::controller::library::slices) fn refresh_exact_duplicate_cleanup_beat_count(
        &mut self,
    ) {
        if self.ui.waveform.slice_batch_profile != WaveformSliceBatchProfile::ExactDuplicateBeats {
            self.ui.waveform.slice_batch_beat_count = 0;
            return;
        }
        self.ui.waveform.slice_batch_beat_count = self
            .ui
            .waveform
            .duplicate_cleanup
            .as_ref()
            .map(|state| {
                state
                    .previews
                    .iter()
                    .filter(|preview| !preview.exempted)
                    .map(|preview| preview.represented_window_count)
                    .sum()
            })
            .unwrap_or(0);
    }

    /// Focus one duplicate cleanup preview and keep slice review active.
    pub(crate) fn focus_duplicate_cleanup_preview(&mut self, index: usize) -> bool {
        if self.ui.waveform.slice_batch_profile != WaveformSliceBatchProfile::ExactDuplicateBeats
            || index >= self.ui.waveform.slices.len()
        {
            return false;
        }
        if !self.ui.waveform.slice_review.active {
            self.ui.waveform.slice_review.active = true;
        }
        self.ui.waveform.slice_review.focused_index = Some(index);
        self.ensure_selection_visible_in_view(self.ui.waveform.slices[index]);
        self.focus_waveform_context();
        self.set_status(self.slice_review_hint(), StatusTone::Info);
        true
    }

    /// Focus and audition one duplicate cleanup preview immediately.
    pub(crate) fn audition_duplicate_cleanup_preview(&mut self, index: usize) -> bool {
        if !self.focus_duplicate_cleanup_preview(index) {
            return false;
        }
        self.play_from_start()
    }

    /// Synchronize visible slice previews from duplicate cleanup state.
    pub(in crate::app::controller::library::slices) fn sync_duplicate_cleanup_previews(&mut self) {
        let Some(cleanup) = self.ui.waveform.duplicate_cleanup.as_ref() else {
            self.ui.waveform.slices.clear();
            self.ui.waveform.slice_batch_beat_count = 0;
            return;
        };
        self.ui.waveform.slices = cleanup
            .previews
            .iter()
            .map(|preview| preview.range)
            .collect();
        self.ui
            .waveform
            .selected_slices
            .retain(|index| *index < self.ui.waveform.slices.len());
        self.refresh_exact_duplicate_cleanup_beat_count();
        self.refresh_slice_review_state();
    }

    pub(crate) fn current_duplicate_cleanup_counts(&self) -> DuplicateCleanupCounts {
        let Some(cleanup) = self.ui.waveform.duplicate_cleanup.as_ref() else {
            return DuplicateCleanupCounts::default();
        };
        let mut counts = DuplicateCleanupCounts {
            group_count: cleanup.group_count,
            ..Default::default()
        };
        for preview in &cleanup.previews {
            if preview.exempted {
                counts.exempted_windows += preview.represented_window_count;
            } else {
                counts.marked_windows += preview.represented_window_count;
            }
        }
        counts
    }
}
