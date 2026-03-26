use super::AppController;
use super::MIN_SELECTION_WIDTH;
use super::selection_export::SelectionEntryRecordRequest;
use crate::analysis::audio::detect_non_silent_ranges_for_slices;
use crate::app::controller::StatusTone;
use crate::app::controller::playback::audio_samples::{
    DecodedSamples, crop_samples, decode_samples_from_bytes, write_wav,
};
use crate::app::state::WaveformSliceBatchProfile;
use crate::sample_sources::SampleSource;
use crate::selection::SelectionRange;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

mod ops;

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

    /// Export detected slices to new audio files and register them in the browser.
    pub(crate) fn accept_waveform_slices(&mut self) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        if self.ui.waveform.slices.is_empty() {
            return Err("No slices to export".into());
        }
        let (source, relative_path, decoded) = self.slice_export_context()?;
        let profile = self.ui.waveform.slice_batch_profile;
        let slices = self.waveform_slice_export_ranges()?;
        let mut counter = 1usize;
        let exported = self.export_slice_batch(
            &source,
            &relative_path,
            &decoded,
            &slices,
            profile,
            &mut counter,
        )?;
        self.clear_waveform_slices();
        Ok(exported)
    }

    /// Return whether keyboard-first slice review is active for the waveform.
    pub(crate) fn slice_review_active(&self) -> bool {
        self.ui.waveform.slice_review.active
    }

    /// Return the currently focused review slice range, if any.
    pub(crate) fn focused_slice_review_range(&self) -> Option<SelectionRange> {
        let index = self.ui.waveform.slice_review.focused_index?;
        self.ui.waveform.slices.get(index).copied()
    }

    /// Start keyboard slice review with the first slice focused and no export marks.
    pub(crate) fn start_slice_review(&mut self) -> bool {
        if self.ui.waveform.slices.is_empty() {
            self.ui.waveform.slice_review = Default::default();
            return false;
        }
        self.ui.waveform.slice_review.active = true;
        self.ui.waveform.slice_review.focused_index = Some(0);
        self.ui.waveform.slice_review.marked_indices.clear();
        self.ensure_selection_visible_in_view(self.ui.waveform.slices[0]);
        self.focus_waveform_context();
        self.set_status(self.slice_review_hint(), StatusTone::Info);
        true
    }

    /// Exit keyboard slice review while preserving any existing slice previews and marks.
    pub(crate) fn exit_slice_review(&mut self) -> bool {
        if !self.ui.waveform.slice_review.active {
            return false;
        }
        self.ui.waveform.slice_review.active = false;
        self.ui.waveform.slice_review.focused_index = None;
        self.set_status("Exited slice review", StatusTone::Info);
        true
    }

    /// Move the focused review slice by one signed delta, clamping at the batch edges.
    pub(crate) fn move_slice_review_focus(&mut self, delta: i8) -> bool {
        if !self.ui.waveform.slice_review.active || self.ui.waveform.slices.is_empty() || delta == 0
        {
            return false;
        }
        let current = self.ui.waveform.slice_review.focused_index.unwrap_or(0);
        let last = self.ui.waveform.slices.len().saturating_sub(1);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current.saturating_add(delta as usize).min(last)
        };
        if next == current {
            self.set_status(
                if current == 0 {
                    "Already at the first slice"
                } else {
                    "Already at the last slice"
                },
                StatusTone::Info,
            );
            return true;
        }
        self.ui.waveform.slice_review.focused_index = Some(next);
        self.ensure_selection_visible_in_view(self.ui.waveform.slices[next]);
        self.focus_waveform_context();
        self.set_status(self.slice_review_hint(), StatusTone::Info);
        true
    }

    /// Toggle export marking on the currently focused review slice.
    pub(crate) fn toggle_focused_slice_export_mark(&mut self) -> Result<bool, String> {
        let index = self
            .ui
            .waveform
            .slice_review
            .focused_index
            .ok_or_else(|| "Focus a slice first".to_string())?;
        if index >= self.ui.waveform.slices.len() {
            return Err("Focus a slice first".to_string());
        }
        if let Some(position) = self
            .ui
            .waveform
            .slice_review
            .marked_indices
            .iter()
            .position(|value| *value == index)
        {
            self.ui
                .waveform
                .slice_review
                .marked_indices
                .swap_remove(position);
            self.ui.waveform.slice_review.marked_indices.sort_unstable();
            self.set_status(
                format!(
                    "Unmarked slice {} for export ({} marked)",
                    index + 1,
                    self.ui.waveform.slice_review.marked_indices.len()
                ),
                StatusTone::Info,
            );
            return Ok(false);
        }
        self.ui.waveform.slice_review.marked_indices.push(index);
        self.ui.waveform.slice_review.marked_indices.sort_unstable();
        self.set_status(
            format!(
                "Marked slice {} for export ({} marked)",
                index + 1,
                self.ui.waveform.slice_review.marked_indices.len()
            ),
            StatusTone::Info,
        );
        Ok(true)
    }

    /// Resolve the slice ranges that should be exported for the current waveform preview batch.
    pub(crate) fn waveform_slice_export_ranges(&self) -> Result<Vec<SelectionRange>, String> {
        if self.ui.waveform.slices.is_empty() {
            return Err("No slices to export".to_string());
        }
        if !self.ui.waveform.slice_review.marked_indices.is_empty() {
            return Ok(self
                .ui
                .waveform
                .slice_review
                .marked_indices
                .iter()
                .filter_map(|index| self.ui.waveform.slices.get(*index).copied())
                .collect());
        }
        if self.ui.waveform.slice_review.active {
            return Err("Mark slices to export first".to_string());
        }
        Ok(self.ui.waveform.slices.clone())
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
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
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
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::Manual;
        self.refresh_slice_review_state();
        Some(merged)
    }

    fn export_slice_batch(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        decoded: &DecodedSamples,
        slices: &[SelectionRange],
        profile: WaveformSliceBatchProfile,
        counter: &mut usize,
    ) -> Result<usize, String> {
        let mut exported = 0usize;
        for slice in slices.iter().copied() {
            self.export_single_slice(source, relative_path, decoded, slice, profile, counter)?;
            exported += 1;
        }
        Ok(exported)
    }

    fn export_single_slice(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        decoded: &DecodedSamples,
        slice: SelectionRange,
        profile: WaveformSliceBatchProfile,
        counter: &mut usize,
    ) -> Result<(), String> {
        let samples = crop_samples(&decoded.samples, decoded.channels, slice)?;
        let target_rel = self.next_slice_path_in_dir(source, relative_path, profile, counter);
        let target_abs = source.root.join(&target_rel);
        write_wav(&target_abs, &samples, decoded.sample_rate, decoded.channels)?;
        self.record_selection_entry(SelectionEntryRecordRequest {
            source,
            relative_path: target_rel,
            target_tag: None,
            add_to_browser: true,
            register_in_source: true,
            looped: false,
            bpm: None,
        })?;
        Ok(())
    }

    fn slice_export_context(&self) -> Result<(SampleSource, PathBuf, DecodedSamples), String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample before exporting slices".to_string())?;
        let decoded = decode_samples_from_bytes(&audio.bytes)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == audio.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        Ok((source, audio.relative_path.clone(), decoded))
    }

    fn next_slice_path_in_dir(
        &self,
        source: &SampleSource,
        original: &Path,
        profile: WaveformSliceBatchProfile,
        counter: &mut usize,
    ) -> PathBuf {
        let parent = original.parent().unwrap_or_else(|| Path::new(""));
        let stem = original
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("slice");
        let stem = match profile {
            WaveformSliceBatchProfile::Manual => strip_numbered_suffix(stem, "slice"),
            WaveformSliceBatchProfile::SilenceSplit => strip_numbered_suffix(stem, "silence_split"),
        };
        loop {
            let suffix = match profile {
                WaveformSliceBatchProfile::Manual => format!("slice{:03}", counter),
                WaveformSliceBatchProfile::SilenceSplit => format!("silence_split_{:03}", counter),
            };
            let candidate = parent.join(format!("{stem}_{suffix}.wav"));
            let absolute = source.root.join(&candidate);
            if !absolute.exists() {
                *counter = counter.saturating_add(1);
                return candidate;
            }
            *counter = counter.saturating_add(1);
        }
    }

    fn refresh_slice_review_state(&mut self) {
        if self.ui.waveform.slices.is_empty() {
            self.ui.waveform.slice_review = Default::default();
            return;
        }
        let max_index = self.ui.waveform.slices.len().saturating_sub(1);
        self.ui
            .waveform
            .slice_review
            .marked_indices
            .retain(|index| *index <= max_index);
        self.ui.waveform.slice_review.marked_indices.sort_unstable();
        self.ui.waveform.slice_review.marked_indices.dedup();
        if self.ui.waveform.slice_review.active {
            let focused = self.ui.waveform.slice_review.focused_index.unwrap_or(0);
            self.ui.waveform.slice_review.focused_index = Some(focused.min(max_index));
        } else {
            self.ui.waveform.slice_review.focused_index = None;
        }
    }

    fn slice_review_hint(&self) -> String {
        let total = self.ui.waveform.slices.len();
        let focused = self
            .ui
            .waveform
            .slice_review
            .focused_index
            .map(|index| index + 1)
            .unwrap_or(1);
        format!("Slice {focused}/{total}. Left/Right review, Space audition, A mark, E export.")
    }
}

fn strip_numbered_suffix<'a>(stem: &'a str, suffix: &str) -> &'a str {
    if let Some((prefix, tail)) = stem.rsplit_once(&format!("_{suffix}")) {
        if !prefix.is_empty() && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return prefix;
        }
    }
    stem
}

#[cfg(test)]
mod ops_tests;
#[cfg(test)]
mod slices_tests;
