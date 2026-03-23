//! Shared waveform hotkey helpers used by both legacy and native dispatch.
//!
//! These helpers keep waveform-specific destructive behaviors close to the
//! controller internals they depend on, while exposing stable `AppController`
//! methods that the native action bridge can call directly.

use super::AppController;
use crate::app::controller::StatusTone;
use crate::app::state::DestructiveSelectionEdit;
use crate::sample_sources::WavEntry;

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
            .find(|source| source.id == audio.source_id)
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
                .and_then(|index| self.wav_entries.entry(index))
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
                Some(f64::from(playhead_position.clamp(0.0, 1.0)))
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

    /// Delete the currently loaded sample and move focus/playback to the next candidate.
    pub(crate) fn delete_loaded_sample_and_navigate(&mut self) -> Result<(), String> {
        use rand::seq::IteratorRandom;

        let (source, relative_path, absolute_path) = {
            let audio = self
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .ok_or_else(|| "No sample loaded to delete".to_string())?;
            let source = self
                .library
                .sources
                .iter()
                .find(|source| source.id == audio.source_id)
                .cloned()
                .ok_or_else(|| "Source not available for loaded sample".to_string())?;
            let relative_path = audio.relative_path.clone();
            let absolute_path = audio.root.join(&audio.relative_path);
            (source, relative_path, absolute_path)
        };

        let next_path = if self.random_navigation_mode_enabled() {
            let total = self.visible_browser_len();
            if total > 1 {
                let mut rng = rand::rng();
                let mut attempts = 0;
                let mut found = None;
                while attempts < 10 {
                    if let Some(row) = (0..total).choose(&mut rng)
                        && let Some(index) = self.visible_browser_index(row)
                        && let Some(entry) = self.wav_entry(index)
                        && entry.relative_path != relative_path
                    {
                        found = Some(entry.relative_path.clone());
                        break;
                    }
                    attempts += 1;
                }
                found
            } else {
                None
            }
        } else if let Some(row) = self.visible_row_for_path(&relative_path) {
            let visible = &self.ui.browser.viewport.visible;
            let next_row = row + 1;
            if next_row < visible.len() {
                visible
                    .get(next_row)
                    .and_then(|index| self.wav_entry(index))
                    .map(|entry| entry.relative_path.clone())
            } else if row > 0 {
                visible
                    .get(row - 1)
                    .and_then(|index| self.wav_entry(index))
                    .map(|entry| entry.relative_path.clone())
            } else {
                None
            }
        } else {
            None
        };

        let context =
            crate::app::controller::library::browser_controller::helpers::TriageSampleContext {
                source,
                entry: WavEntry {
                    relative_path: relative_path.clone(),
                    file_size: 0,
                    modified_ns: 0,
                    content_hash: None,
                    tag: crate::sample_sources::Rating::NEUTRAL,
                    looped: false,
                    locked: false,
                    missing: false,
                    last_played_at: None,
                },
                absolute_path,
            };

        self.browser().try_delete_browser_sample_ctx(&context)?;

        if let Some(path) = next_path {
            if let Some(row) = self.visible_row_for_path(&path) {
                self.focus_browser_row_only(row);
                let loop_enabled = self.ui.waveform.loop_enabled;
                if let Err(err) = self.play_audio(loop_enabled, None) {
                    self.set_status(err, StatusTone::Error);
                }
            } else {
                self.select_wav_by_path_with_rebuild(&path, true);
            }
        } else {
            self.set_status("No more samples to navigate to", StatusTone::Info);
        }

        Ok(())
    }
}
