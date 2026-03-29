use super::*;
use crate::analysis::audio::detect_exact_duplicate_beat_ranges;
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
        let grid_origin_frame = self.current_duplicate_grid_origin_frame(total_frames);
        let detection = detect_exact_duplicate_beat_ranges(
            samples.as_ref(),
            channels,
            sample_rate,
            bpm,
            grid_origin_frame,
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
        self.ui.waveform.slice_batch_beat_count = detection.duplicate_beat_count;
        self.ui.waveform.slice_mode_enabled = true;
        if self.ui.waveform.slices.is_empty() {
            self.clear_waveform_slices();
            self.set_status("No exact duplicate beats found", StatusTone::Info);
            return Ok(0);
        }
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
        let windows = collect_full_beat_windows(
            total_frames,
            beat_frames,
            self.current_duplicate_grid_origin_frame(total_frames),
        );
        let count = windows
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

    fn current_duplicate_grid_origin_frame(&self, total_frames: usize) -> f64 {
        let origin = if self.ui.waveform.relative_bpm_grid_enabled {
            self.ui
                .waveform
                .selection
                .map(|selection| selection.start())
                .unwrap_or(self.ui.waveform.last_bpm_grid_origin)
        } else {
            0.0
        };
        f64::from(origin.clamp(0.0, 1.0)) * total_frames as f64
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
