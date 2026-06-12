use std::path::Path;

use crate::native_app::app::{NativeAppState, WaveformState};

impl NativeAppState {
    pub(super) fn clear_loaded_sample_if_exact(&mut self, path: &Path) {
        if self.waveform.current.path() == path {
            self.clear_loaded_sample_after_trash();
        }
    }

    pub(super) fn clear_loaded_sample_if_path_within(&mut self, root: &Path) {
        let loaded_path = self.waveform.current.path();
        if !loaded_path.as_os_str().is_empty() && loaded_path.starts_with(root) {
            self.clear_loaded_sample_after_trash();
        }
    }

    fn clear_loaded_sample_after_trash(&mut self) {
        if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
        self.waveform.current = WaveformState::empty();
        self.audio.current_playback_span = None;
    }
}
