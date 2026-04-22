use super::*;
use std::path::Path;

/// High-level browser selection and focus facade methods on `AppController`.
impl AppController {
    /// Select a wav row based on its path.
    pub fn select_wav_by_path(&mut self, path: &Path) {
        selection_ops::select_wav_by_path(self, path);
    }

    /// Select a wav row based on its path, optionally delaying the browser rebuild.
    pub fn select_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::select_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Focus a wav row by path without queueing audio/waveform loading.
    ///
    /// This supports high-frequency browser focus navigation where loading is
    /// committed separately by an explicit action.
    pub(crate) fn focus_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by path while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_path_preview_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_preview_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by absolute index while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_index_preview_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::focus_wav_by_index_preview_with_rebuild(self, index, rebuild);
    }

    /// Select a wav row by absolute index, optionally delaying browser list rebuild.
    pub(crate) fn select_wav_by_index_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::select_wav_by_index_with_rebuild(self, index, rebuild);
    }

    /// Map the current browser filter into a drop target tag for drag-and-drop retagging.
    pub fn triage_flag_drop_target(&self) -> TriageFlagColumn {
        selection_ops::triage_flag_drop_target(self)
    }

    /// Current tag of the selected wav, if any.
    pub fn selected_tag(&mut self) -> Option<crate::sample_sources::Rating> {
        selection_ops::selected_tag(self)
    }

    /// Build a library sample_id for the visible browser row.
    pub fn sample_id_for_visible_row(&mut self, row: usize) -> Result<String, String> {
        let source_id = self
            .selection_state
            .ctx
            .selected_source
            .clone()
            .ok_or_else(|| "No active source selected".to_string())?;
        let entry_index = self
            .ui
            .browser
            .viewport
            .visible
            .get(row)
            .ok_or_else(|| "Selected row is out of range".to_string())?;
        let entry = self
            .wav_entry(entry_index)
            .ok_or_else(|| "Sample entry missing".to_string())?;
        Ok(analysis_jobs::build_sample_id(
            source_id.as_str(),
            &entry.relative_path,
        ))
    }

    /// Build a library sample_id for the currently selected wav.
    pub fn selected_sample_id(&self) -> Option<String> {
        let source_id = self.selection_state.ctx.selected_source.as_ref()?;
        let path = self.sample_view.wav.selected_wav.as_ref()?;
        Some(analysis_jobs::build_sample_id(source_id.as_str(), path))
    }

    /// Focus the sample browser on a library sample_id without autoplay.
    pub fn focus_sample_from_map(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source_id = SourceId::from_string(source_id);
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
            self.select_source(Some(source_id.clone()));
        }
        self.focus_browser_context();
        self.ui.browser.selection.autoscroll = true;
        if !self.ui.browser.selection.selected_paths.is_empty() {
            self.clear_browser_selected_indices();
        }
        self.ui.browser.selection.selection_anchor_visible = None;
        self.selection_state.suppress_autoplay_once = true;
        self.select_wav_by_path(&relative_path);
        if let Some(row) = self.visible_row_for_path(&relative_path) {
            self.ui.browser.selection.selection_anchor_visible = Some(row);
        }
        Ok(())
    }

    /// Load waveform/audio for a given library sample_id without requiring browser selection.
    pub fn preview_sample_by_id(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|source| source.id.as_str() == source_id)
            .map(|source| SampleSource {
                id: source.id.clone(),
                root: source.root.clone(),
            })
            .ok_or_else(|| format!("Unknown source for sample_id: {sample_id}"))?;
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source.id) {
            self.select_source(Some(source.id.clone()));
        }
        self.sample_view.wav.selected_wav = Some(relative_path.clone());
        self.queue_audio_load_for(&source, &relative_path, AudioLoadIntent::Selection, None)
    }

    /// Select a wav by absolute index into the full wav list.
    pub fn select_wav_by_index(&mut self, index: usize) {
        selection_ops::select_wav_by_index(self, index);
    }

    /// Select a wav coming from the sample browser and clear collection focus.
    pub fn select_from_browser(&mut self, path: &Path) {
        selection_ops::select_from_browser(self, path);
    }
}
