use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use wavecrate::selection::SelectionRange;

use super::{
    WaveformState,
    audio_file::{extract_wav_range_to_folder, extract_wav_range_to_sibling, is_wav_path},
};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformExtractionCompletion {
    pub(in crate::native_app) source_path: PathBuf,
    pub(in crate::native_app) selection: SelectionRange,
    pub(in crate::native_app) result: Result<PathBuf, String>,
}

pub(in crate::native_app) struct WaveformExtractionRequest {
    source_path: PathBuf,
    target_folder: Option<PathBuf>,
    audio_bytes: Arc<[u8]>,
    loaded_frames: usize,
    selection: SelectionRange,
}

impl WaveformState {
    #[cfg(test)]
    pub(in crate::native_app) fn extract_play_selection_to_sibling(
        &mut self,
    ) -> Result<PathBuf, String> {
        let selection = self.extractable_play_selection()?;
        let path = extract_wav_range_to_sibling(
            &self.file.path,
            &self.file.audio_bytes,
            self.file.frames,
            selection,
        )?;
        self.mark_extracted_range(selection);
        Ok(path)
    }

    #[cfg(test)]
    pub(in crate::native_app) fn extract_play_selection_to_folder(
        &mut self,
        target_folder: &Path,
    ) -> Result<PathBuf, String> {
        let selection = self.extractable_play_selection()?;
        let path = extract_wav_range_to_folder(
            &self.file.path,
            target_folder,
            &self.file.audio_bytes,
            self.file.frames,
            selection,
        )?;
        self.mark_extracted_range(selection);
        Ok(path)
    }

    pub(in crate::native_app) fn play_selection_extraction_request(
        &self,
        target_folder: Option<PathBuf>,
    ) -> Result<WaveformExtractionRequest, String> {
        let selection = self.extractable_play_selection()?;
        Ok(WaveformExtractionRequest {
            source_path: self.file.path.clone(),
            target_folder,
            audio_bytes: Arc::clone(&self.file.audio_bytes),
            loaded_frames: self.file.frames,
            selection,
        })
    }

    pub(in crate::native_app) fn mark_extracted_play_selection(
        &mut self,
        source_path: &Path,
        selection: SelectionRange,
    ) {
        if self.file.path == source_path {
            self.mark_extracted_range(selection);
        }
    }

    fn mark_extracted_range(&mut self, selection: SelectionRange) {
        if selection.width() <= 0.0 {
            return;
        }
        self.extracted_ranges
            .push(SelectionRange::new(selection.start(), selection.end()));
        self.extracted_ranges
            .sort_by(|a, b| a.start_f64().total_cmp(&b.start_f64()));

        let mut merged = Vec::with_capacity(self.extracted_ranges.len());
        for range in self.extracted_ranges.drain(..) {
            let Some(previous) = merged.last_mut() else {
                merged.push(range);
                continue;
            };
            if range.start_f64() <= previous.end_f64() + 1.0e-6 {
                *previous = SelectionRange::new_precise(
                    previous.start_f64(),
                    previous.end_f64().max(range.end_f64()),
                );
            } else {
                merged.push(range);
            }
        }
        self.extracted_ranges = merged;
    }

    fn extractable_play_selection(&self) -> Result<SelectionRange, String> {
        let selection = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
            .ok_or_else(|| String::from("Mark a play range before extracting"))?;
        if !self.has_loaded_sample() {
            return Err(String::from("Load a sample before extracting"));
        }
        if self.file.audio_bytes.is_empty() {
            return Err(String::from(
                "Reload the sample before extracting from a playback cache",
            ));
        }
        if !is_wav_path(&self.file.path) {
            return Err(String::from("Extraction currently supports WAV files"));
        }
        Ok(selection)
    }

    pub(in crate::native_app) fn play_selection(&self) -> Option<SelectionRange> {
        self.play_selection
    }

    pub(in crate::native_app) fn edit_selection(&self) -> Option<SelectionRange> {
        self.edit_selection
    }

    pub(in crate::native_app) fn extracted_ranges(&self) -> &[SelectionRange] {
        &self.extracted_ranges
    }
}

pub(in crate::native_app) fn execute_waveform_extraction(
    request: WaveformExtractionRequest,
) -> WaveformExtractionCompletion {
    let result = match request.target_folder.as_deref() {
        Some(target_folder) => extract_wav_range_to_folder(
            &request.source_path,
            target_folder,
            &request.audio_bytes,
            request.loaded_frames,
            request.selection,
        ),
        None => extract_wav_range_to_sibling(
            &request.source_path,
            &request.audio_bytes,
            request.loaded_frames,
            request.selection,
        ),
    };
    WaveformExtractionCompletion {
        source_path: request.source_path,
        selection: request.selection,
        result,
    }
}
