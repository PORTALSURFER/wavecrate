use super::*;
use crate::app::state::{LoopCrossfadePrompt, LoopCrossfadeSettings};
use std::path::{Path, PathBuf};

mod audio;
mod file_output;
mod undo_ops;

#[cfg(test)]
mod tests;

impl AppController {
    /// Open the loop crossfade prompt for a visible browser row.
    pub fn request_loop_crossfade_prompt_for_browser_row(
        &mut self,
        row: usize,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        self.ui.loop_crossfade_prompt = Some(LoopCrossfadePrompt {
            source_id: ctx.source.id,
            relative_path: ctx.entry.relative_path,
            settings: LoopCrossfadeSettings::default(),
        });
        Ok(())
    }

    /// Apply the pending loop crossfade prompt.
    pub fn apply_loop_crossfade_prompt(&mut self) -> Result<(), String> {
        let Some(prompt) = self.ui.loop_crossfade_prompt.clone() else {
            return Ok(());
        };
        self.ui.loop_crossfade_prompt = None;
        let source = loop_crossfade_source(self, &prompt.source_id)?;
        let absolute_path = source.root.join(&prompt.relative_path);
        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;

        let new_relative = self.apply_loop_crossfade_for_sample(
            &source,
            &prompt.relative_path,
            &absolute_path,
            &prompt.settings,
        )?;

        if was_playing {
            let start_override = if playhead_position.is_finite() {
                Some(playhead_position.clamp(0.0, 1.0))
            } else {
                None
            };
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: source.id.clone(),
                    relative_path: new_relative.clone(),
                    looped: was_looping,
                    start_override,
                }));
            // Suppress the default autoplay to avoid double-trigger or reset to start.
            self.selection_state.suppress_autoplay_once = true;
        }

        self.select_from_browser(&new_relative);
        Ok(())
    }

    /// Clear any pending loop crossfade prompt.
    pub fn clear_loop_crossfade_prompt(&mut self) {
        self.ui.loop_crossfade_prompt = None;
    }

    /// Apply a loop crossfade copy for a single sample path.
    pub(crate) fn apply_loop_crossfade_for_sample(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        absolute_path: &Path,
        settings: &LoopCrossfadeSettings,
    ) -> Result<PathBuf, String> {
        let rendered = audio::render_loop_crossfade(absolute_path, settings)?;
        let tag = self.sample_tag_for(source, relative_path)?;
        let output =
            file_output::write_loop_crossfade_copy(&source.root, relative_path, &rendered)?;
        file_output::register_loop_crossfade_entry(self, source, &output, tag)?;
        undo_ops::maybe_capture_loop_crossfade_undo(self, source, &output, tag);
        self.set_status(
            format!("Created loop crossfade {}", output.relative_path.display()),
            StatusTone::Info,
        );
        Ok(output.relative_path)
    }
}

/// Resolve the sample source referenced by a loop crossfade prompt or undo entry.
fn loop_crossfade_source(
    controller: &AppController,
    source_id: &SourceId,
) -> Result<SampleSource, String> {
    controller
        .library
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .cloned()
        .ok_or_else(|| "Source not available".to_string())
}
