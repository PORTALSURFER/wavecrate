use super::HotkeysController;
use crate::app::controller::StatusTone;
use crate::sample_sources::WavEntry;

impl HotkeysController<'_> {
    /// Delete the currently loaded sample and move focus/playback to the next candidate.
    pub(super) fn delete_loaded_sample_and_navigate(&mut self) -> Result<(), String> {
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
                .find(|s| s.id == audio.source_id)
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
                        && let Some(idx) = self.visible_browser_index(row)
                        && let Some(entry) = self.wav_entry(idx)
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
                    .and_then(|idx| self.wav_entry(idx))
                    .map(|entry| entry.relative_path.clone())
            } else if row > 0 {
                visible
                    .get(row - 1)
                    .and_then(|idx| self.wav_entry(idx))
                    .map(|entry| entry.relative_path.clone())
            } else {
                None
            }
        } else {
            None
        };

        let ctx =
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

        self.browser().try_delete_browser_sample_ctx(&ctx)?;

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
