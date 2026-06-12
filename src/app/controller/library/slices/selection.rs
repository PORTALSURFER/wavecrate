use super::super::AppController;
use super::duplicate_preview;
use super::ops;
use crate::app::state::WaveformSliceBatchProfile;
use crate::selection::SelectionRange;

impl AppController {
    /// Toggle the selection state for a slice index.
    pub(crate) fn toggle_slice_selection(&mut self, index: usize) -> bool {
        if self.loaded_waveform_slice_export_in_progress() {
            return false;
        }
        if index >= self.ui.waveform.slices.len() {
            return false;
        }
        if let Some(pos) = self
            .ui
            .waveform
            .selected_slices
            .iter()
            .position(|value| *value == index)
        {
            self.ui.waveform.selected_slices.swap_remove(pos);
            return false;
        }
        self.ui.waveform.selected_slices.push(index);
        self.ui.waveform.selected_slices.sort_unstable();
        true
    }

    /// Remove selected slice ranges from the waveform.
    pub(crate) fn delete_selected_slices(&mut self) -> usize {
        if self.loaded_waveform_slice_export_in_progress() {
            return 0;
        }
        if self.ui.waveform.selected_slices.is_empty() {
            return 0;
        }
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats
            && let Some(cleanup) = self.ui.waveform.duplicate_cleanup.as_mut()
        {
            let (previews, removed) = duplicate_preview::delete_duplicate_previews(
                &cleanup.previews,
                &self.ui.waveform.selected_slices,
            );
            cleanup.previews = previews;
            if cleanup.previews.is_empty() {
                self.clear_waveform_slices();
                return removed;
            }
            self.ui.waveform.selected_slices.clear();
            self.sync_duplicate_cleanup_previews();
            return removed;
        }

        let deleted =
            ops::delete_slices(&self.ui.waveform.slices, &self.ui.waveform.selected_slices);
        self.ui.waveform.slices = deleted.slices;
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            return deleted.removed;
        }
        self.ui.waveform.selected_slices.clear();
        self.refresh_slice_batch_after_collection_mutation();
        deleted.removed
    }

    /// Merge selected slice ranges into a single range that spans them.
    pub(crate) fn merge_selected_slices(&mut self) -> Option<SelectionRange> {
        if self.loaded_waveform_slice_export_in_progress() {
            return None;
        }
        if self.ui.waveform.selected_slices.len() < 2 {
            return None;
        }
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats
            && let Some(cleanup) = self.ui.waveform.duplicate_cleanup.as_mut()
        {
            let merged = duplicate_preview::merge_duplicate_previews(
                &cleanup.previews,
                &self.ui.waveform.selected_slices,
            )?;
            cleanup.previews = merged.previews;
            self.sync_duplicate_cleanup_previews();
            let merged_index = self
                .ui
                .waveform
                .slices
                .iter()
                .position(|slice| *slice == merged.merged)
                .unwrap_or(0);
            self.ui.waveform.selected_slices = vec![merged_index];
            self.refresh_slice_review_state();
            return Some(merged.merged);
        }

        let merged = ops::merge_selected_slices(
            &self.ui.waveform.slices,
            &self.ui.waveform.selected_slices,
        )?;
        self.ui.waveform.slices = merged.slices;
        self.ui.waveform.selected_slices = merged.selected_indices;
        self.refresh_slice_batch_after_collection_mutation();
        Some(merged.merged)
    }

    fn refresh_slice_batch_after_collection_mutation(&mut self) {
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            self.refresh_exact_duplicate_cleanup_beat_count();
        } else {
            self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
            self.ui.waveform.slice_batch_beat_count = 0;
        }
        self.refresh_slice_review_state();
    }
}
