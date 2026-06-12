use super::super::AppController;
use super::super::MIN_SELECTION_WIDTH;
use super::ops;
use crate::app::state::WaveformSliceBatchProfile;
use crate::selection::SelectionRange;

impl AppController {
    /// Clear any detected slice ranges from the waveform view.
    pub(crate) fn clear_waveform_slices(&mut self) {
        self.ui.waveform.slices.clear();
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_review = Default::default();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.ui.waveform.slice_batch_beat_count = 0;
        self.ui.waveform.duplicate_cleanup = None;
    }

    /// Apply a manually painted slice, cutting it out of any overlapping slices.
    pub(crate) fn apply_painted_slice(&mut self, range: SelectionRange) -> bool {
        if self.loaded_waveform_slice_export_in_progress() {
            return false;
        }
        let Some(updated) =
            ops::apply_painted_slice(&self.ui.waveform.slices, range, MIN_SELECTION_WIDTH)
        else {
            return false;
        };
        self.ui.waveform.slices = updated;
        self.reset_manual_slice_batch();
        self.refresh_slice_review_state();
        true
    }

    /// Update an existing slice range, cutting it out of any overlapping slices.
    pub(crate) fn update_slice_range(
        &mut self,
        index: usize,
        range: SelectionRange,
    ) -> Option<usize> {
        if self.loaded_waveform_slice_export_in_progress() {
            return None;
        }
        let updated = ops::update_slice_range(
            &self.ui.waveform.slices,
            &self.ui.waveform.selected_slices,
            index,
            range,
            MIN_SELECTION_WIDTH,
        )?;
        self.ui.waveform.slices = updated.slices;
        self.ui.waveform.selected_slices = updated.selected_indices;
        self.reset_manual_slice_batch();
        self.refresh_slice_review_state();
        updated.new_index
    }

    /// Snap a slice paint position to BPM or transient markers when enabled.
    pub(crate) fn snap_slice_paint_position(&self, position: f32, snap_override: bool) -> f32 {
        let state = ops::SliceSnapState {
            bpm_snap_enabled: self.ui.waveform.bpm_snap_enabled,
            bpm_value: self.ui.waveform.bpm_value,
            duration_seconds: self
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .map(|audio| audio.duration_seconds),
            transient_markers_enabled: self.ui.waveform.transient_markers_enabled,
            transient_snap_enabled: self.ui.waveform.transient_snap_enabled,
            transients: self.ui.waveform.transients.to_vec(),
        };
        ops::snap_slice_paint_position(&state, position, snap_override)
    }

    pub(super) fn reset_manual_slice_batch(&mut self) {
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.ui.waveform.slice_batch_beat_count = 0;
        self.ui.waveform.duplicate_cleanup = None;
    }
}
