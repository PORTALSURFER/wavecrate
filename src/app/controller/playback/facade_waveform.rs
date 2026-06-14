use super::*;

impl AppController {
    /// Apply a committed playback selection range and refresh dependent labels/preview state.
    pub(crate) fn apply_selection(&mut self, range: Option<SelectionRange>) {
        player::apply_selection(self, range);
    }

    /// Apply the edit-selection overlay range used by waveform fade/trim editing tools.
    pub(crate) fn apply_edit_selection(&mut self, range: Option<SelectionRange>) {
        player::apply_edit_selection(self, range);
    }

    /// Update the hover time indicator for the waveform.
    pub fn update_waveform_hover_time(&mut self, position: Option<f32>) {
        player::update_waveform_hover_time(self, position);
    }

    #[cfg(test)]
    pub(crate) fn selection_duration_label(&self, range: SelectionRange) -> Option<String> {
        player::selection_duration_label(self, range)
    }

    /// Apply output volume to runtime audio state without persisting configuration.
    pub(crate) fn apply_volume(&mut self, volume: f32) {
        player::apply_volume(self, volume);
    }
}
