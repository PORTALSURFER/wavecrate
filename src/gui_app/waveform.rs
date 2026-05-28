#![allow(missing_docs)]

use radiant::prelude as ui;
use std::{path::PathBuf, sync::Arc};

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const MIN_VISIBLE_FRAMES: usize = 256;
const BAND_COUNT: usize = 4;
const SELECTION_DRAG_EPSILON: f32 = 0.001;
const SELECTION_FLASH_FRAMES: u8 = 12;
#[cfg(test)]
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
#[cfg(test)]
const SYNTHETIC_SECONDS: usize = 1;

#[derive(Clone, Debug)]
pub(super) struct WaveformState {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    zoom_anchor_ratio: f32,
    playing: bool,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    active_drag: Option<WaveformDrag>,
    pending_playback_start: Option<f32>,
}

impl WaveformState {
    pub(super) fn load_default() -> Result<Self, String> {
        Ok(Self::empty())
    }

    pub(super) fn load_path(path: PathBuf) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file(path)?);
        Ok(Self::from_file(file))
    }

    pub(super) fn load_path_with_progress(
        path: PathBuf,
        progress: impl Fn(f32),
    ) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file_with_progress(path, progress)?);
        Ok(Self::from_file(file))
    }

    pub(super) fn from_cached_file(file: Arc<WaveformFile>) -> Self {
        Self::from_file(file)
    }

    pub(super) fn empty() -> Self {
        Self::from_file(Arc::new(empty_waveform_file()))
    }

    #[cfg(test)]
    pub(super) fn synthetic_for_tests() -> Self {
        Self::from_file(Arc::new(synthetic_waveform_file()))
    }

    fn from_file(file: Arc<WaveformFile>) -> Self {
        let viewport = WaveformViewport::full(file.frames);
        Self {
            file,
            viewport,
            zoom_anchor_ratio: 0.5,
            playing: false,
            playhead_ratio: None,
            play_mark_ratio: None,
            edit_mark_ratio: None,
            play_selection: None,
            edit_selection: None,
            extracted_ranges: Vec::new(),
            play_selection_flash_frames: 0,
            active_drag: None,
            pending_playback_start: None,
        }
    }

    pub(super) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(super) fn file(&self) -> Arc<WaveformFile> {
        Arc::clone(&self.file)
    }

    pub(super) fn viewport(&self) -> WaveformViewport {
        self.viewport
    }

    pub(super) fn playhead_ratio(&self) -> Option<f32> {
        self.playhead_ratio
    }

    pub(super) fn play_mark_ratio(&self) -> Option<f32> {
        self.play_mark_ratio
    }

    pub(super) fn edit_mark_ratio(&self) -> Option<f32> {
        self.edit_mark_ratio
    }

    pub(super) fn play_selection(&self) -> Option<wavecrate::selection::SelectionRange> {
        self.play_selection
    }

    pub(super) fn edit_selection(&self) -> Option<wavecrate::selection::SelectionRange> {
        self.edit_selection
    }

    pub(super) fn extracted_ranges(&self) -> &[wavecrate::selection::SelectionRange] {
        &self.extracted_ranges
    }

    pub(super) fn play_selection_flash_frames(&self) -> u8 {
        self.play_selection_flash_frames
    }

    pub(super) fn play_selection_flash_active(&self) -> bool {
        self.play_selection_flash_frames > 0
    }

    pub(super) fn flash_play_selection(&mut self) {
        self.play_selection_flash_frames = SELECTION_FLASH_FRAMES;
    }

    pub(super) fn extract_play_selection_to_sibling(&mut self) -> Result<PathBuf, String> {
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

    pub(super) fn extract_play_selection_to_folder(
        &mut self,
        target_folder: &std::path::Path,
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

    fn mark_extracted_range(&mut self, selection: wavecrate::selection::SelectionRange) {
        if selection.width() <= 0.0 {
            return;
        }
        self.extracted_ranges
            .push(wavecrate::selection::SelectionRange::new(
                selection.start(),
                selection.end(),
            ));
        self.extracted_ranges
            .sort_by(|a, b| a.start_f64().total_cmp(&b.start_f64()));

        let mut merged = Vec::with_capacity(self.extracted_ranges.len());
        for range in self.extracted_ranges.drain(..) {
            let Some(previous) = merged.last_mut() else {
                merged.push(range);
                continue;
            };
            if range.start_f64() <= previous.end_f64() + 1.0e-6 {
                *previous = wavecrate::selection::SelectionRange::new_precise(
                    previous.start_f64(),
                    previous.end_f64().max(range.end_f64()),
                );
            } else {
                merged.push(range);
            }
        }
        self.extracted_ranges = merged;
    }

    fn extractable_play_selection(&self) -> Result<wavecrate::selection::SelectionRange, String> {
        let selection = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
            .ok_or_else(|| String::from("Mark a play range before extracting"))?;
        if !self.has_loaded_sample() {
            return Err(String::from("Load a sample before extracting"));
        }
        if !is_wav_path(&self.file.path) {
            return Err(String::from("Extraction currently supports WAV files"));
        }
        Ok(selection)
    }

    pub(super) fn active_drag_kind(&self) -> Option<WaveformActiveDragKind> {
        self.active_drag.map(WaveformDrag::kind)
    }

    pub(super) fn take_pending_playback_start(&mut self) -> Option<f32> {
        self.pending_playback_start.take()
    }

    pub(super) fn start_playback(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playing = true;
        self.play_mark_ratio = Some(ratio);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(super) fn set_playhead_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(super) fn stop_playback(&mut self) {
        self.playing = false;
        self.playhead_ratio = None;
    }

    pub(super) fn sample_rate(&self) -> u32 {
        self.file.sample_rate
    }

    pub(super) fn channels(&self) -> usize {
        self.file.channels
    }

    pub(super) fn frames(&self) -> usize {
        self.file.frames
    }

    pub(super) fn file_name(&self) -> String {
        if self.file.path.as_os_str().is_empty() {
            return String::from("No sample loaded");
        }
        self.file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.file.path.display().to_string())
    }

    pub(super) fn path(&self) -> PathBuf {
        self.file.path.clone()
    }

    pub(super) fn rewrite_path_prefix(
        &mut self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> bool {
        if self.file.path == old_path {
            Arc::make_mut(&mut self.file).path = new_path.to_path_buf();
            return true;
        }
        if let Ok(relative) = self.file.path.strip_prefix(old_path) {
            Arc::make_mut(&mut self.file).path = new_path.join(relative);
            return true;
        }
        false
    }

    pub(super) fn has_loaded_sample(&self) -> bool {
        !self.file.audio_bytes.is_empty() && !self.file.path.as_os_str().is_empty()
    }

    pub(super) fn audio_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.file.audio_bytes)
    }

    pub(super) fn visible_fraction(&self) -> f32 {
        self.viewport.visible_fraction(self.file.frames)
    }

    pub(super) fn fully_zoomed_out(&self) -> bool {
        self.viewport
            .clamp(self.file.frames, MIN_VISIBLE_FRAMES)
            .visible_items()
            >= self.file.frames.max(1)
    }

    pub(super) fn offset_fraction(&self) -> f32 {
        self.viewport.offset_fraction(self.file.frames)
    }
}

mod types;
pub(super) use types::{
    WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionEdge,
    WaveformSelectionKind,
};

mod interaction;
use interaction::{WaveformDrag, edit_preview_for_selection};

mod state_interaction;
mod state_selection;
mod state_viewport;

mod audio_file;
pub(super) use audio_file::WaveformFile;
#[cfg(test)]
use audio_file::{
    downmix_to_mono, split_frequency_bands, synthetic_waveform_file,
    waveform_file_from_mono_samples,
};
use audio_file::{
    empty_waveform_file, extract_wav_range_to_folder, extract_wav_range_to_sibling, is_wav_path,
    load_waveform_file, load_waveform_file_with_progress,
};

mod signal_widget;
use signal_widget::WaveformSignalWidget;

mod widget;
#[cfg(test)]
pub(super) use widget::WaveformWidgetProps;
pub(super) use widget::{WaveformWidget, waveform_viewport_view};

mod widget_geometry;
mod widget_input;

mod edit_fade_curve_paint;
mod edit_fade_geometry;
mod edit_fade_paint;
mod paint_primitives;
mod selection_paint;

pub(super) type WaveformViewport = ui::IndexViewport;

#[cfg(test)]
mod tests;
