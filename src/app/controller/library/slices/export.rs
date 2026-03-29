use super::*;
use crate::app::controller::library::selection_export::SelectionEntryRecordRequest;
use crate::app::controller::playback::audio_samples::{
    DecodedSamples, crop_samples, decode_samples_from_bytes, write_wav,
};
use crate::sample_sources::SampleSource;
use std::path::{Path, PathBuf};

impl AppController {
    /// Export detected slices to new audio files and register them in the browser.
    pub(crate) fn accept_waveform_slices(&mut self) -> Result<usize, String> {
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            return Err("Use Clean Dups to apply exact duplicate cleanup".to_string());
        }
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

    pub(super) fn next_slice_path_in_dir(
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
            WaveformSliceBatchProfile::ExactDuplicateBeats => {
                strip_numbered_suffix(stem, "exact_duplicate")
            }
        };
        loop {
            let suffix = match profile {
                WaveformSliceBatchProfile::Manual => format!("slice{:03}", counter),
                WaveformSliceBatchProfile::SilenceSplit => format!("silence_split_{:03}", counter),
                WaveformSliceBatchProfile::ExactDuplicateBeats => {
                    format!("exact_duplicate_{:03}", counter)
                }
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
}

fn strip_numbered_suffix<'a>(stem: &'a str, suffix: &str) -> &'a str {
    if let Some((prefix, tail)) = stem.rsplit_once(&format!("_{suffix}")) {
        if !prefix.is_empty() && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return prefix;
        }
    }
    stem
}
