use super::*;
use crate::app::state::{AudioPickerTarget, OptionsPanelPrompt};

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
        self.ui.options_panel.pending_prompt = None;
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
            self.ui.options_panel.pending_prompt = None;
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

    /// Open the prompt used to edit the default auto-rename identifier.
    pub(crate) fn start_default_identifier_prompt(&mut self) {
        self.open_options_panel();
        self.ui.options_panel.pending_prompt = Some(OptionsPanelPrompt::DefaultIdentifier {
            value: self.settings.default_identifier.clone(),
        });
    }

    /// Update the active options-panel prompt input, if any.
    pub(crate) fn set_options_panel_prompt_input(&mut self, value: String) -> bool {
        if let Some(OptionsPanelPrompt::DefaultIdentifier { value: current }) =
            self.ui.options_panel.pending_prompt.as_mut()
        {
            *current = value;
            return true;
        }
        false
    }

    /// Return whether the options panel currently owns a modal prompt.
    pub(crate) fn has_pending_options_panel_prompt(&self) -> bool {
        self.ui.options_panel.pending_prompt.is_some()
    }

    /// Confirm the active options-panel prompt, if present.
    pub(crate) fn apply_pending_options_panel_prompt(&mut self) {
        let Some(OptionsPanelPrompt::DefaultIdentifier { value }) =
            self.ui.options_panel.pending_prompt.clone()
        else {
            return;
        };
        let normalized = if value.trim().is_empty() {
            String::from("portal")
        } else {
            value.trim().to_string()
        };
        self.settings.default_identifier = normalized.clone();
        self.ui.options_panel.default_identifier = normalized.clone();
        self.ui.options_panel.pending_prompt = None;
        if let Err(err) = self.persist_config("Failed to save default identifier") {
            self.set_status(err, StatusTone::Error);
            return;
        }
        self.set_status(
            format!("Default identifier set to {normalized}"),
            StatusTone::Info,
        );
    }

    /// Cancel the active options-panel prompt, if present.
    pub(crate) fn cancel_options_panel_prompt(&mut self) {
        self.ui.options_panel.pending_prompt = None;
    }
}
