use super::AppController;
use super::MIN_SELECTION_WIDTH;
use crate::analysis::audio::detect_non_silent_ranges_for_slices;
use crate::app::controller::StatusTone;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use crate::app::state::WaveformSliceBatchProfile;
use crate::selection::SelectionRange;
use std::borrow::Cow;
use std::cmp::Ordering;

mod exact_duplicates;
mod export;
mod ops;
mod review;

impl AppController {
    /// Detect silence-split slice ranges for the loaded waveform and store them in UI state.
    pub(crate) fn detect_waveform_slices_from_silence(&mut self) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample before slicing".to_string())?;
        let (samples, sample_rate, channels) =
            if let Some(decoded) = self.sample_view.waveform.decoded.as_ref() {
                if decoded.peaks.is_none() && !decoded.samples.is_empty() {
                    (
                        Cow::Borrowed(decoded.samples.as_ref()),
                        decoded.sample_rate.max(1),
                        decoded.channels.max(1),
                    )
                } else {
                    let decoded = decode_samples_from_bytes(&audio.bytes)?;
                    (
                        Cow::Owned(decoded.samples),
                        decoded.sample_rate.max(1),
                        decoded.channels.max(1),
                    )
                }
            } else {
                let decoded = decode_samples_from_bytes(&audio.bytes)?;
                (
                    Cow::Owned(decoded.samples),
                    decoded.sample_rate.max(1),
                    decoded.channels.max(1),
                )
            };
        let total_frames = samples.len() / channels.max(1) as usize;
        if total_frames == 0 {
            return Err("No audio data to slice".into());
        }
        let slices = detect_non_silent_ranges_for_slices(samples.as_ref(), channels, sample_rate)
            .into_iter()
            .map(|(start, end)| {
                let start_norm = start as f32 / total_frames as f32;
                let end_norm = end as f32 / total_frames as f32;
                SelectionRange::new(start_norm, end_norm)
            })
            .filter(|range| range.width() >= MIN_SELECTION_WIDTH)
            .collect::<Vec<_>>();
        self.ui.waveform.slices = slices;
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;
        self.ui.waveform.slice_batch_beat_count = 0;
        self.ui.waveform.slice_mode_enabled = true;
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            self.set_status("No audible slices found", StatusTone::Info);
            return Ok(0);
        }
        self.start_slice_review();
        Ok(self.ui.waveform.slices.len())
    }

    /// Run silence-only slice detection and surface any failure via status UI.
    pub(crate) fn detect_waveform_silence_slices_action(&mut self) {
        if self.loaded_waveform_slice_export_in_progress() {
            self.set_status(
                "Wait for the current slice export to finish",
                StatusTone::Info,
            );
            self.focus_waveform_context();
            return;
        }
        if let Err(err) = self.detect_waveform_slices_from_silence() {
            self.set_error_status(err);
        }
        self.focus_waveform_context();
    }

    /// Clear any detected slice ranges from the waveform view.
    pub(crate) fn clear_waveform_slices(&mut self) {
        self.ui.waveform.slices.clear();
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_review = Default::default();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.ui.waveform.slice_batch_beat_count = 0;
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
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.ui.waveform.slice_batch_beat_count = 0;
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
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.ui.waveform.slice_batch_beat_count = 0;
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
        let mut indices = self.ui.waveform.selected_slices.clone();
        indices.sort_unstable();
        indices.dedup();
        let mut removed = 0usize;
        for index in indices.into_iter().rev() {
            if index < self.ui.waveform.slices.len() {
                self.ui.waveform.slices.remove(index);
                removed += 1;
            }
        }
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            return removed;
        }
        self.ui.waveform.selected_slices.clear();
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            self.refresh_exact_duplicate_cleanup_beat_count();
        } else {
            self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
            self.ui.waveform.slice_batch_beat_count = 0;
        }
        self.refresh_slice_review_state();
        removed
    }

    /// Merge selected slice ranges into a single range that spans them.
    pub(crate) fn merge_selected_slices(&mut self) -> Option<SelectionRange> {
        if self.loaded_waveform_slice_export_in_progress() {
            return None;
        }
        if self.ui.waveform.selected_slices.len() < 2 {
            return None;
        }
        let mut indices = self.ui.waveform.selected_slices.clone();
        indices.sort_unstable();
        indices.dedup();
        let mut min_start: f32 = 1.0;
        let mut max_end: f32 = 0.0;
        for &index in &indices {
            if let Some(slice) = self.ui.waveform.slices.get(index) {
                min_start = min_start.min(slice.start());
                max_end = max_end.max(slice.end());
            }
        }
        if max_end <= min_start {
            return None;
        }
        let merged = SelectionRange::new(min_start, max_end);
        self.ui.waveform.slices = self
            .ui
            .waveform
            .slices
            .iter()
            .copied()
            .filter(|slice| !ops::ranges_overlap(*slice, merged))
            .collect();
        self.ui.waveform.slices.push(merged);
        self.ui
            .waveform
            .slices
            .sort_by(|a, b| a.start().partial_cmp(&b.start()).unwrap_or(Ordering::Equal));
        let merged_index = self
            .ui
            .waveform
            .slices
            .iter()
            .position(|slice| *slice == merged)
            .unwrap_or(0);
        self.ui.waveform.selected_slices = vec![merged_index];
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            self.refresh_exact_duplicate_cleanup_beat_count();
        } else {
            self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
            self.ui.waveform.slice_batch_beat_count = 0;
        }
        self.refresh_slice_review_state();
        Some(merged)
    }
}

#[cfg(test)]
mod ops_tests;
#[cfg(test)]
mod slices_tests;
