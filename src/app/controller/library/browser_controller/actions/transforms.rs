use super::*;

impl BrowserController<'_> {
    pub(super) fn normalize_browser_sample_action(&mut self, row: usize) -> Result<(), String> {
        let result = self.try_normalize_browser_sample(row);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    pub(super) fn normalize_browser_samples_action(
        &mut self,
        rows: &[usize],
    ) -> Result<(), String> {
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
        if self.warn_if_any_browser_context_busy(&contexts, "normalizing") {
            return Ok(());
        }
        for ctx in contexts {
            if let Err(err) = self.try_normalize_browser_sample_ctx(&ctx) {
                last_error = Some(err);
            }
        }
        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }

    pub(super) fn loop_crossfade_browser_samples_action(
        &mut self,
        rows: &[usize],
        settings: LoopCrossfadeSettings,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
        let primary_path = self
            .resolve_browser_sample(primary_visible_row)
            .ok()
            .map(|ctx| ctx.entry.relative_path);

        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;
        let primary_is_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                primary_path
                    .as_ref()
                    .is_some_and(|path| audio.relative_path == *path)
            });

        let mut primary_new = None;
        let mut primary_source = None;
        for ctx in contexts {
            match self.apply_loop_crossfade_for_sample(
                &ctx.source,
                &ctx.entry.relative_path,
                &ctx.absolute_path,
                &settings,
            ) {
                Ok(new_relative) => {
                    if primary_path
                        .as_ref()
                        .is_some_and(|path| path == &ctx.entry.relative_path)
                    {
                        primary_new = Some(new_relative);
                        primary_source = Some(ctx.source.clone());
                    }
                }
                Err(err) => last_error = Some(err),
            }
        }
        if let Some(path) = primary_new {
            if primary_is_loaded
                && was_playing
                && let Some(source) = primary_source
            {
                let start_override = if playhead_position.is_finite() {
                    Some(f64::from(playhead_position.clamp(0.0, 1.0)))
                } else {
                    None
                };
                self.runtime
                    .jobs
                    .set_pending_playback(Some(PendingPlayback {
                        source_id: source.id,
                        relative_path: path.clone(),
                        looped: was_looping,
                        start_override,
                        force_loaded_audio: false,
                    }));
                self.selection_state.suppress_autoplay_once = true;
            }
            self.select_from_browser(&path);
        }
        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}
