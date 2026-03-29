use super::*;
use crate::analysis::audio::{DetectedDuplicateWindow, detect_exact_duplicate_window_ranges};
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use crate::app::state::{
    WaveformDuplicateCleanupPreview, WaveformDuplicateCleanupState, WaveformSliceBatchProfile,
};
use std::borrow::Cow;

impl AppController {
    /// Detect near-duplicate hit windows across the loaded waveform using the current selection size.
    pub(crate) fn detect_waveform_exact_duplicate_slices_from_selection(
        &mut self,
    ) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        let (samples, sample_rate, channels) = self.waveform_slice_analysis_audio()?;
        let total_frames = samples.len() / channels.max(1) as usize;
        if total_frames == 0 {
            return Err("No audio data to scan".to_string());
        }
        let scan = self
            .current_duplicate_window_scan_config(total_frames)
            .ok_or_else(|| {
                "Create a playback selection to define the duplicate window size".to_string()
            })?;
        let transient_frames = self.current_duplicate_candidate_event_frames(total_frames);
        let detection = detect_exact_duplicate_window_ranges(
            samples.as_ref(),
            channels,
            sample_rate,
            scan.window_frames,
            scan.anchor_start_frame,
            &transient_frames,
        )?;
        if detection.duplicate_windows.is_empty() {
            self.clear_waveform_slices();
            self.set_status(
                "No near-duplicate windows found for the current selection size",
                StatusTone::Info,
            );
            return Ok(0);
        }

        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
        self.ui.waveform.slice_mode_enabled = true;
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.duplicate_cleanup = Some(build_duplicate_cleanup_state(
            &detection.duplicate_windows,
            detection.duplicate_group_count,
            total_frames,
        ));
        self.sync_duplicate_cleanup_previews();
        self.start_slice_review();
        Ok(self.ui.waveform.slices.len())
    }

    /// Run duplicate window detection and surface any failure via status UI.
    pub(crate) fn detect_waveform_exact_duplicate_slices_action(&mut self) {
        if self.loaded_waveform_slice_export_in_progress() {
            self.set_status(
                "Wait for the current slice export to finish",
                StatusTone::Info,
            );
            self.focus_waveform_context();
            return;
        }
        if let Err(err) = self.detect_waveform_exact_duplicate_slices_from_selection() {
            self.set_error_status(err);
        }
        self.focus_waveform_context();
    }

    /// Keep duplicate cleanup counts synchronized with the current visible preview batch.
    pub(super) fn refresh_exact_duplicate_cleanup_beat_count(&mut self) {
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

    /// Toggle whether one duplicate cleanup preview should be excluded from cleanup.
    pub(crate) fn toggle_duplicate_cleanup_preview_exemption(
        &mut self,
        index: usize,
    ) -> Result<bool, String> {
        let cleanup = self
            .ui
            .waveform
            .duplicate_cleanup
            .as_mut()
            .ok_or_else(|| "Run Exact Dedupe before editing duplicate cleanup".to_string())?;
        let preview = cleanup
            .previews
            .get_mut(index)
            .ok_or_else(|| "Select a duplicate cleanup preview first".to_string())?;
        preview.exempted = !preview.exempted;
        let exempted = preview.exempted;
        self.sync_duplicate_cleanup_previews();
        self.focus_duplicate_cleanup_preview(
            index.min(self.ui.waveform.slices.len().saturating_sub(1)),
        );
        let counts = self.current_duplicate_cleanup_counts();
        self.set_status(
            if exempted {
                format!(
                    "Keeping duplicate {}/{} for now. {} marked, {} kept, {} group(s).",
                    index + 1,
                    self.ui.waveform.slices.len(),
                    counts.marked_windows,
                    counts.exempted_windows,
                    counts.group_count
                )
            } else {
                format!(
                    "Marked duplicate {}/{} for cleanup. {} marked, {} kept, {} group(s).",
                    index + 1,
                    self.ui.waveform.slices.len(),
                    counts.marked_windows,
                    counts.exempted_windows,
                    counts.group_count
                )
            },
            StatusTone::Info,
        );
        Ok(exempted)
    }

    /// Synchronize visible slice previews from duplicate cleanup state.
    pub(super) fn sync_duplicate_cleanup_previews(&mut self) {
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

    fn current_duplicate_window_scan_config(
        &self,
        total_frames: usize,
    ) -> Option<DuplicateWindowScanConfig> {
        let selection = self.ui.waveform.selection?;
        let (anchor_start_frame, anchor_end_frame) =
            selection_frame_bounds(total_frames, selection);
        let window_frames = anchor_end_frame.saturating_sub(anchor_start_frame);
        (window_frames > 0).then_some(DuplicateWindowScanConfig {
            anchor_start_frame,
            window_frames,
        })
    }

    fn current_duplicate_candidate_event_frames(&self, total_frames: usize) -> Vec<usize> {
        let mut frames = self
            .ui
            .waveform
            .transients
            .iter()
            .copied()
            .map(|value| {
                ((value.clamp(0.0, 1.0) * total_frames as f32).round() as usize).min(total_frames)
            })
            .filter(|frame| *frame < total_frames)
            .collect::<Vec<_>>();
        frames.sort_unstable();
        frames.dedup();
        frames
    }

    fn waveform_slice_analysis_audio(&self) -> Result<(Cow<'_, [f32]>, u32, u16), String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample before slicing".to_string())?;
        if let Some(decoded) = self.sample_view.waveform.decoded.as_ref() {
            if decoded.peaks.is_none() && !decoded.samples.is_empty() {
                return Ok((
                    Cow::Borrowed(decoded.samples.as_ref()),
                    decoded.sample_rate.max(1),
                    decoded.channels.max(1),
                ));
            }
        }
        let decoded = decode_samples_from_bytes(&audio.bytes)?;
        Ok((
            Cow::Owned(decoded.samples),
            decoded.sample_rate.max(1),
            decoded.channels.max(1),
        ))
    }
}

#[derive(Clone, Copy)]
struct DuplicateWindowScanConfig {
    anchor_start_frame: usize,
    window_frames: usize,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DuplicateCleanupCounts {
    pub(crate) group_count: usize,
    pub(crate) marked_windows: usize,
    pub(crate) exempted_windows: usize,
}

fn build_duplicate_cleanup_state(
    windows: &[DetectedDuplicateWindow],
    duplicate_group_count: usize,
    total_frames: usize,
) -> WaveformDuplicateCleanupState {
    WaveformDuplicateCleanupState {
        group_count: duplicate_group_count,
        previews: windows
            .iter()
            .map(|window| WaveformDuplicateCleanupPreview {
                range: SelectionRange::new(
                    window.start_frame as f32 / total_frames as f32,
                    window.end_frame as f32 / total_frames as f32,
                ),
                group_id: window.group_id,
                exempted: false,
                represented_window_count: 1,
            })
            .collect(),
    }
}

fn selection_frame_bounds(total_frames: usize, bounds: SelectionRange) -> (usize, usize) {
    let start_frame = ((bounds.start() * total_frames as f32).floor() as usize)
        .min(total_frames.saturating_sub(1));
    let mut end_frame = ((bounds.end() * total_frames as f32).ceil() as usize).min(total_frames);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    (start_frame, end_frame)
}
