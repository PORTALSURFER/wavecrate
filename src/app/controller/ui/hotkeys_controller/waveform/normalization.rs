use super::HotkeysController;
use crate::app::controller::{AppController, StatusTone};
use crate::app::state::DestructiveSelectionEdit;
use crate::sample_sources::WavEntry;

impl HotkeysController<'_> {
    /// Normalize the active waveform selection, or fall back to the loaded sample.
    pub(super) fn normalize_waveform_selection_or_sample_action(&mut self) {
        AppController::normalize_waveform_selection_or_sample_action(self);
    }
}

impl AppController {
    /// Normalize the active waveform selection, or the loaded sample when no selection exists.
    pub(crate) fn normalize_waveform_selection_or_sample_action(&mut self) {
        if matches!(self.ui.waveform.selection, Some(selection) if selection.width() > 0.0) {
            let _ = self
                .request_destructive_selection_edit(DestructiveSelectionEdit::NormalizeSelection);
            return;
        }
        if let Err(err) = self.normalize_loaded_sample_like_browser() {
            self.set_status(err, StatusTone::Error);
        }
    }

    /// Normalize the currently loaded sample in-place while preserving waveform state.
    pub(crate) fn normalize_loaded_sample_like_browser(&mut self) -> Result<(), String> {
        let preserved_view = self.ui.waveform.view;
        let preserved_cursor = self.ui.waveform.cursor;
        let preserved_selection = self.ui.waveform.selection;
        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample to normalize it".to_string())?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == audio.source_id)
            .cloned()
            .ok_or_else(|| "Source not available for loaded sample".to_string())?;
        let relative_path = audio.relative_path.clone();
        let absolute_path = source.root.join(&relative_path);
        let (file_size, modified_ns, tag) =
            self.normalize_and_save_for_path(&source, &relative_path, &absolute_path)?;
        self.upsert_metadata_for_source(&source, &relative_path, file_size, modified_ns)?;
        let last_played_at = self
            .sample_last_played_for(&source, &relative_path)
            .unwrap_or(None);
        let looped = self
            .sample_looped_for(&source, &relative_path)
            .unwrap_or(false);
        let updated = WavEntry {
            relative_path: relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped,
            locked: self
                .wav_index_for_path(&relative_path)
                .and_then(|idx| self.wav_entries.entry(idx))
                .map(|entry| entry.locked)
                .unwrap_or(false),
            missing: false,
            last_played_at,
        };
        self.update_cached_entry(&source, &relative_path, updated);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
            self.rebuild_browser_lists();
        }
        self.refresh_waveform_for_sample(&source, &relative_path);
        self.ui.waveform.view = preserved_view.clamp();
        self.ui.waveform.cursor = preserved_cursor;
        self.selection_state.range.set_range(preserved_selection);
        self.apply_selection(preserved_selection);
        if was_playing {
            let start_override = if playhead_position.is_finite() {
                Some(playhead_position.clamp(0.0, 1.0))
            } else {
                None
            };
            if let Err(err) = self.play_audio(was_looping, start_override) {
                self.set_status(err, StatusTone::Error);
            }
        }
        self.set_status(
            format!("Normalized {}", relative_path.display()),
            StatusTone::Info,
        );
        Ok(())
    }
}
