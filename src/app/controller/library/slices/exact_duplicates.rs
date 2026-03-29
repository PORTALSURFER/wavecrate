use super::*;
use crate::analysis::audio::{ExactDuplicateBeatDetection, detect_exact_duplicate_beat_ranges};
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use std::borrow::Cow;

impl AppController {
    /// Detect exact BPM-aligned duplicate beat ranges for the loaded waveform.
    pub(crate) fn detect_waveform_exact_duplicate_slices_from_bpm(
        &mut self,
    ) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        let bpm = self
            .ui
            .waveform
            .bpm_value
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
            .ok_or_else(|| "Set a valid BPM value before cleaning duplicates".to_string())?;
        let (samples, sample_rate, channels) = self.waveform_slice_analysis_audio()?;
        let total_frames = samples.len() / channels.max(1) as usize;
        if total_frames == 0 {
            return Err("No audio data to scan".to_string());
        }
        let (detection, anchor_label) =
            self.best_exact_duplicate_detection(samples.as_ref(), channels, sample_rate, bpm)?;
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
        self.ui.waveform.slice_batch_beat_count = detection.duplicate_beat_count;
        self.ui.waveform.slice_mode_enabled = true;
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            self.set_status(
                format!(
                    "No exact duplicate beats found at {bpm:.1} BPM. Align a playmark or selection to the beat grid and try again."
                ),
                StatusTone::Info,
            );
            return Ok(0);
        }
        self.set_status(
            format!(
                "Found {} duplicate beat(s) across {} cleanup range(s) using {anchor_label}",
                self.ui.waveform.slice_batch_beat_count,
                self.ui.waveform.slices.len(),
            ),
            StatusTone::Info,
        );
        self.start_slice_review();
        Ok(self.ui.waveform.slices.len())
    }

    /// Run exact-duplicate beat detection and surface any failure via status UI.
    pub(crate) fn detect_waveform_exact_duplicate_slices_action(&mut self) {
        if self.loaded_waveform_slice_export_in_progress() {
            self.set_status(
                "Wait for the current slice export to finish",
                StatusTone::Info,
            );
            self.focus_waveform_context();
            return;
        }
        if let Err(err) = self.detect_waveform_exact_duplicate_slices_from_bpm() {
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
            .current_exact_duplicate_cleanup_beat_count(&self.ui.waveform.slices)
            .unwrap_or(0);
    }

    fn current_exact_duplicate_cleanup_beat_count(
        &self,
        slices: &[SelectionRange],
    ) -> Option<usize> {
        if slices.is_empty() {
            return Some(0);
        }
        let bpm = self.ui.waveform.bpm_value?;
        if !bpm.is_finite() || bpm <= 0.0 {
            return Some(0);
        }
        let (samples, sample_rate, channels) = self.waveform_slice_analysis_audio().ok()?;
        let total_frames = samples.len() / channels.max(1) as usize;
        if total_frames == 0 || sample_rate == 0 {
            return Some(0);
        }
        let beat_frames = sample_rate as f64 * 60.0 / f64::from(bpm);
        if !beat_frames.is_finite() || beat_frames < 1.0 {
            return Some(0);
        }
        let count = self
            .current_duplicate_grid_origins(total_frames)
            .into_iter()
            .map(|(origin, _)| {
                collect_full_beat_windows(total_frames, beat_frames, origin)
                    .into_iter()
                    .filter(|window| {
                        slices.iter().copied().any(|slice| {
                            let start = slice.start() * total_frames as f32;
                            let end = slice.end() * total_frames as f32;
                            start <= window.0 as f32 && end >= window.1 as f32
                        })
                    })
                    .count()
            })
            .max()
            .unwrap_or(0);
        Some(count)
    }

    fn best_exact_duplicate_detection(
        &self,
        samples: &[f32],
        channels: u16,
        sample_rate: u32,
        bpm: f32,
    ) -> Result<(ExactDuplicateBeatDetection, &'static str), String> {
        let total_frames = samples.len() / channels.max(1) as usize;
        let mut best = None;
        for (origin_frame, label) in self.current_duplicate_grid_origins(total_frames) {
            let detection = detect_exact_duplicate_beat_ranges(
                samples,
                channels,
                sample_rate,
                bpm,
                origin_frame,
            )?;
            let score = (
                detection.duplicate_beat_count,
                detection.duplicate_ranges.len(),
            );
            if best
                .as_ref()
                .is_none_or(|(best_score, _, _)| score > *best_score)
            {
                best = Some((score, detection, label));
            }
        }
        let (_, detection, label) = best
            .ok_or_else(|| "No BPM grid anchors are available for duplicate cleanup".to_string())?;
        Ok((detection, label))
    }

    fn current_duplicate_grid_origins(&self, total_frames: usize) -> Vec<(f64, &'static str)> {
        let mut origins = Vec::new();
        if self.ui.waveform.relative_bpm_grid_enabled {
            if let Some(selection) = self.ui.waveform.selection {
                origins.push((
                    f64::from(selection.start().clamp(0.0, 1.0)) * total_frames as f64,
                    "selection start",
                ));
            }
            origins.push((
                f64::from(self.ui.waveform.last_bpm_grid_origin.clamp(0.0, 1.0))
                    * total_frames as f64,
                "relative BPM grid",
            ));
        }
        if let Some(start_marker) = self.ui.waveform.last_start_marker {
            origins.push((
                f64::from(start_marker.clamp(0.0, 1.0)) * total_frames as f64,
                "playmark",
            ));
        }
        origins.push((0.0, "sample start"));

        let mut unique = Vec::new();
        for (origin, label) in origins {
            if unique.iter().any(|(existing, _): &(f64, &'static str)| {
                (existing - origin).abs() <= f64::EPSILON
            }) {
                continue;
            }
            unique.push((origin, label));
        }
        unique
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

fn collect_full_beat_windows(
    total_frames: usize,
    beat_frames: f64,
    grid_origin_frame: f64,
) -> Vec<(usize, usize)> {
    let mut windows = Vec::new();
    let mut beat_index = ((-grid_origin_frame) / beat_frames).floor() as i64 - 1;
    let total_frames_i64 = total_frames as i64;

    loop {
        let start_frame = (grid_origin_frame + beat_index as f64 * beat_frames).round() as i64;
        let end_frame = (grid_origin_frame + (beat_index + 1) as f64 * beat_frames).round() as i64;
        if start_frame >= total_frames_i64 {
            break;
        }
        if start_frame >= 0 && end_frame <= total_frames_i64 && end_frame > start_frame {
            windows.push((start_frame as usize, end_frame as usize));
        }
        beat_index += 1;
    }

    windows
}
