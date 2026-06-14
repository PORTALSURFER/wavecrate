use super::super::AppController;
use super::super::MIN_SELECTION_WIDTH;
use crate::app::controller::StatusTone;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use crate::app::state::WaveformSliceBatchProfile;
use crate::selection::SelectionRange;
use std::borrow::Cow;
use wavecrate_analysis::detect_non_silent_ranges_for_slices;

impl AppController {
    /// Detect silence-split slice ranges for the loaded waveform and store them in UI state.
    pub(crate) fn detect_waveform_slices_from_silence(&mut self) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        let (samples, sample_rate, channels) = self.silence_slice_analysis_audio()?;
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
        self.ui.waveform.duplicate_cleanup = None;
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

    fn silence_slice_analysis_audio(&self) -> Result<(Cow<'_, [f32]>, u32, u16), String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample before slicing".to_string())?;
        if let Some(decoded) = self.sample_view.waveform.decoded.as_ref()
            && decoded.peaks.is_none()
            && !decoded.samples.is_empty()
        {
            return Ok((
                Cow::Borrowed(decoded.samples.as_ref()),
                decoded.sample_rate.max(1),
                decoded.channels.max(1),
            ));
        }
        let decoded = decode_samples_from_bytes(&audio.bytes)?;
        Ok((
            Cow::Owned(decoded.samples),
            decoded.sample_rate.max(1),
            decoded.channels.max(1),
        ))
    }
}
