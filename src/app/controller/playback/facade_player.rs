use super::*;
use std::cell::RefCell;
#[cfg(test)]
use std::path::PathBuf;
use std::rc::Rc;

impl AppController {
    /// Returns the duration in seconds for the currently loaded audio, if any.
    pub(crate) fn loaded_audio_duration_seconds(&self) -> Option<f32> {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.duration_seconds)
    }

    /// Start playback with the provided loop and start override settings.
    pub fn play_audio(&mut self, looped: bool, start_override: Option<f64>) -> Result<(), String> {
        player::play_audio(self, looped, start_override)
    }

    /// Return true if audio playback is currently active.
    pub fn is_playing(&self) -> bool {
        player::is_playing(self)
    }

    /// Return live player progress while transport is actively running.
    ///
    /// UI projection paths use this to derive smoother motion-only playhead
    /// updates than the coarser retained UI snapshot alone can provide.
    pub(crate) fn live_playback_progress(&self) -> Option<f32> {
        player::live_progress(self)
    }

    /// Advance the playhead position based on playback progress.
    pub fn tick_playhead(&mut self) {
        player::tick_playhead(self);
    }

    #[cfg(test)]
    pub(crate) fn update_playhead_from_progress(
        &mut self,
        progress: Option<f32>,
        is_looping: bool,
    ) {
        player::update_playhead_from_progress(self, progress, is_looping, false);
    }

    /// Hide the waveform playhead/selection overlays when playback is inactive.
    pub(crate) fn hide_waveform_playhead(&mut self) {
        player::hide_waveform_playhead(self);
    }

    #[cfg(test)]
    pub(crate) fn playhead_completed_span_for_tests(
        &self,
        progress: f32,
        is_looping: bool,
    ) -> bool {
        player::playhead_completed_span_for_tests(self, progress, is_looping)
    }

    #[cfg(test)]
    pub(crate) fn hide_waveform_playhead_for_tests(&mut self) {
        player::hide_waveform_playhead_for_tests(self);
    }

    #[cfg(test)]
    /// Seed a minimal loaded-audio fixture for tests that only need duration metadata.
    pub(crate) fn set_loaded_audio_duration_for_tests(&mut self, duration_seconds: f32) {
        self.sample_view.wav.loaded_audio = (duration_seconds.is_finite()
            && duration_seconds > 0.0)
            .then_some(crate::app::controller::state::audio::LoadedAudio {
                source_id: crate::sample_sources::SourceId::new(),
                root: PathBuf::new(),
                relative_path: PathBuf::from("test.wav"),
                bytes: Vec::new().into(),
                duration_seconds,
                sample_rate: 48_000,
                channels: 1,
            });
    }

    /// Lazily create or return the shared audio player used by playback operations.
    pub(crate) fn ensure_player(&mut self) -> Result<Option<Rc<RefCell<AudioPlayer>>>, String> {
        player::ensure_player(self)
    }
}
