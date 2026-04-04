use super::*;
use crate::app::state::AudioPickerTarget;

impl AppController {
    /// Open the native-shell options panel.
    pub fn open_options_panel(&mut self) {
        self.ensure_startup_audio_refresh();
        self.ui.options_panel.open = true;
        self.ui.options_panel.active_audio_picker = None;
    }

    /// Close the native-shell options panel.
    pub fn close_options_panel(&mut self) {
        self.ui.options_panel.open = false;
        self.ui.options_panel.active_audio_picker = None;
    }

    /// Toggle the native-shell options panel visibility.
    pub fn toggle_options_panel(&mut self) {
        if !self.ui.options_panel.open {
            self.ensure_startup_audio_refresh();
            self.ui.options_panel.active_audio_picker = None;
        }
        self.ui.options_panel.open = !self.ui.options_panel.open;
        if !self.ui.options_panel.open {
            self.ui.options_panel.active_audio_picker = None;
        }
    }

    /// Show the audio-options overview inside the native-shell options panel.
    pub fn show_audio_options_overview(&mut self) {
        self.ui.options_panel.active_audio_picker = None;
    }

    /// Expand one audio picker inside the native-shell options panel.
    pub fn open_audio_picker(&mut self, picker: AudioPickerTarget) {
        self.open_options_panel();
        self.ui.options_panel.active_audio_picker = Some(picker);
    }
}
