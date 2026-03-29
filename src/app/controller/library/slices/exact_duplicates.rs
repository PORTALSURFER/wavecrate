use super::*;
use crate::analysis::audio::detect_exact_duplicate_window_ranges;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use std::borrow::Cow;

impl AppController {
    /// Detect exact duplicate windows across the loaded waveform using the current selection size.
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
        let detection = detect_exact_duplicate_window_ranges(
            samples.as_ref(),
            channels,
            sample_rate,
            scan.window_frames,
            scan.anchor_start_frame,
        )?;
        self.ui.waveform.slices = detection
            .duplicate_ranges
            .into_iter()
            .map(|(start, end)| {
                SelectionRange::new(
                    start as f32 / total_frames as f32,
                    end as f32 / total_frames as f32,
                )
            })
            .collect();
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
        self.ui.waveform.slice_batch_beat_count = detection.duplicate_window_count;
        self.ui.waveform.slice_mode_enabled = true;
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            self.set_status(
                "No exact duplicate windows found for the current selection size",
                StatusTone::Info,
            );
            return Ok(0);
        }
        self.start_slice_review();
        Ok(self.ui.waveform.slices.len())
    }

    /// Run exact-duplicate window detection and surface any failure via status UI.
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

    pub(super) fn refresh_exact_duplicate_cleanup_beat_count(&mut self) {
        if self.ui.waveform.slice_batch_profile != WaveformSliceBatchProfile::ExactDuplicateBeats {
            self.ui.waveform.slice_batch_beat_count = 0;
            return;
        }
        self.ui.waveform.slice_batch_beat_count = self
            .current_exact_duplicate_cleanup_window_count(&self.ui.waveform.slices)
            .unwrap_or(0);
    }

    fn current_exact_duplicate_cleanup_window_count(
        &self,
        slices: &[SelectionRange],
    ) -> Option<usize> {
        if slices.is_empty() {
            return Some(0);
        }
        let (samples, _, channels) = self.waveform_slice_analysis_audio().ok()?;
        let total_frames = samples.len() / channels.max(1) as usize;
        let scan = self.current_duplicate_window_scan_config(total_frames)?;
        let count = collect_full_windows(scan.anchor_start_frame, scan.window_frames, total_frames)
            .into_iter()
            .filter(|window| {
                slices.iter().copied().any(|slice| {
                    let start = slice.start() * total_frames as f32;
                    let end = slice.end() * total_frames as f32;
                    start <= window.0 as f32 && end >= window.1 as f32
                })
            })
            .count();
        Some(count)
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

fn collect_full_windows(
    anchor_start_frame: usize,
    window_frames: usize,
    total_frames: usize,
) -> Vec<(usize, usize)> {
    if window_frames == 0 || window_frames > total_frames {
        return Vec::new();
    }
    let mut start_frame = anchor_start_frame.min(total_frames.saturating_sub(1));
    while start_frame >= window_frames {
        start_frame -= window_frames;
    }
    let mut windows = Vec::new();
    while start_frame + window_frames <= total_frames {
        windows.push((start_frame, start_frame + window_frames));
        start_frame += window_frames;
    }
    windows
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
