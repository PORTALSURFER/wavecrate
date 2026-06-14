use super::super::super::*;

impl AppController {
    pub(in crate::app::controller::library::sources) fn clear_wavs(&mut self) {
        self.wav_entries.clear();
        self.clear_all_folder_projection_state();
        self.sample_view.wav.selected_wav = None;
        self.runtime.similarity.pending_filter_rebuild = None;
        self.clear_focused_similarity_highlight();
        self.ui.browser = SampleBrowserState::default();
        self.ui.sources.folders = FolderBrowserUiState::default();
        self.sync_active_folder_ui_to_pane();
        self.clear_waveform_view();
        if let Some(selected) = self.selection_state.ctx.selected_source.as_ref() {
            self.library.missing.wavs.remove(selected);
        } else {
            self.library.missing.wavs.clear();
        }
    }
}
