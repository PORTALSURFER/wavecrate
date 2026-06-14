use std::{path::PathBuf, sync::Arc};

#[cfg(test)]
use super::audio_file::load_cached_waveform_file_for_playback;
#[cfg(test)]
use super::audio_file::load_waveform_file_with_progress_cancel_and_playback_ready;
#[cfg(test)]
use super::audio_file::synthetic_waveform_file;
use super::{
    MIN_VISIBLE_FRAMES, SELECTION_FLASH_FRAMES, WaveformDrag, WaveformFile, WaveformState,
    WaveformViewport,
    audio_file::{
        empty_waveform_file, load_waveform_file, load_waveform_file_for_foreground_audition,
        load_waveform_file_with_progress_and_cancel,
    },
};

impl WaveformState {
    pub(in crate::native_app) fn load_default() -> Result<Self, String> {
        Ok(Self::empty())
    }

    pub(in crate::native_app) fn load_path(path: PathBuf) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file(path)?);
        Ok(Self::from_file(file))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn load_persisted_playback_cache(
        path: PathBuf,
    ) -> Result<Self, String> {
        let file = load_cached_waveform_file_for_playback(path.clone())
            .ok_or_else(|| format!("playback-ready waveform cache miss: {}", path.display()))?;
        Ok(Self::from_file(Arc::new(file)))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn load_path_with_progress(
        path: PathBuf,
        progress: impl Fn(f32),
    ) -> Result<Self, String> {
        Self::load_path_with_progress_and_cancel(path, progress, || false)
    }

    pub(in crate::native_app) fn load_path_with_progress_and_cancel(
        path: PathBuf,
        progress: impl Fn(f32),
        cancelled: impl Fn() -> bool,
    ) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file_with_progress_and_cancel(
            path, progress, cancelled,
        )?);
        Ok(Self::from_file(file))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn load_path_with_progress_cancel_and_playback_ready(
        path: PathBuf,
        progress: impl Fn(f32),
        cancelled: impl Fn() -> bool,
        playback_ready: impl Fn(super::WaveformPlaybackReady),
    ) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file_with_progress_cancel_and_playback_ready(
            path,
            progress,
            cancelled,
            playback_ready,
        )?);
        Ok(Self::from_file(file))
    }

    pub(in crate::native_app) fn load_path_for_foreground_audition(
        path: PathBuf,
        progress: impl Fn(f32),
        cancelled: impl Fn() -> bool,
        playback_ready: impl Fn(super::WaveformPlaybackReady),
    ) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file_for_foreground_audition(
            path,
            progress,
            cancelled,
            playback_ready,
        )?);
        Ok(Self::from_file(file))
    }

    pub(in crate::native_app) fn from_cached_file(file: Arc<WaveformFile>) -> Self {
        Self::from_file(file)
    }

    pub(in crate::native_app) fn empty() -> Self {
        Self::from_file(Arc::new(empty_waveform_file()))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn synthetic_for_tests() -> Self {
        Self::from_file(Arc::new(synthetic_waveform_file()))
    }

    pub(super) fn from_file(file: Arc<WaveformFile>) -> Self {
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
            marked_play_ranges: Vec::new(),
            extracted_ranges: Vec::new(),
            play_selection_flash_frames: 0,
            active_drag: None::<WaveformDrag>,
            pending_playback_start: None,
        }
    }

    pub(super) fn viewport_scope(&self) -> super::ui::IndexViewportScope {
        super::ui::IndexViewportScope::new(self.viewport, self.file.frames, MIN_VISIBLE_FRAMES)
    }

    pub(super) fn selection_flash_frame_count() -> u8 {
        SELECTION_FLASH_FRAMES
    }
}
