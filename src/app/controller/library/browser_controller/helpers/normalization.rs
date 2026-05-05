//! Browser normalization entrypoints and sync-test helpers.

use super::*;
use crate::app::controller::library::wav_io::ensure_wav_destructive_edit_target;

impl BrowserController<'_> {
    /// Normalize the browser row at `row`, preserving undo and playback resume semantics.
    pub(crate) fn try_normalize_browser_sample(&mut self, row: usize) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        self.try_normalize_browser_sample_ctx(&ctx)
    }

    /// Normalize the resolved browser sample if it is not blocked by retained-delete recovery.
    pub(crate) fn try_normalize_browser_sample_ctx(
        &mut self,
        ctx: &TriageSampleContext,
    ) -> Result<(), String> {
        ensure_wav_destructive_edit_target(&ctx.absolute_path, "Normalize overwrite")?;
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "normalizing",
        ) {
            return Ok(());
        }
        if cfg!(test) {
            return self.normalize_browser_sample_sync(ctx);
        }
        self.controller.begin_pending_sample_overwrite_transaction(
            crate::app::controller::history::PendingHistoryTransactionKey::Normalization {
                source_id: ctx.source.id.clone(),
                relative_path: ctx.entry.relative_path.clone(),
            },
            format!("Normalized {}", ctx.entry.relative_path.display()),
            ctx.source.id.clone(),
            ctx.entry.relative_path.clone(),
            ctx.absolute_path.clone(),
        )?;
        let job = NormalizationJob {
            source: ctx.source.clone(),
            relative_path: ctx.entry.relative_path.clone(),
            absolute_path: ctx.absolute_path.clone(),
        };

        if !self
            .controller
            .ui
            .progress
            .has_task(ProgressTaskKind::Normalization)
        {
            self.controller.show_status_progress(
                ProgressTaskKind::Normalization,
                format!("Normalizing {}", ctx.entry.relative_path.display()),
                1,
                false,
            );
        }

        self.controller.runtime.jobs.begin_normalization(job);
        Ok(())
    }

    fn normalize_browser_sample_sync(&mut self, ctx: &TriageSampleContext) -> Result<(), String> {
        let before = self.capture_meaningful_ui_snapshot();
        let backup = undo::OverwriteBackup::capture_before(&ctx.absolute_path)?;
        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;

        let (file_size, modified_ns, tag) = self.normalize_and_save_for_path(
            &ctx.source,
            &ctx.entry.relative_path,
            &ctx.absolute_path,
        )?;
        let entry_index = self.wav_index_for_path(&ctx.entry.relative_path);
        let looped = entry_index
            .and_then(|idx| self.wav_entries.entry(idx))
            .map(|entry| entry.looped)
            .unwrap_or(false);
        let last_played_at = entry_index
            .and_then(|idx| self.wav_entries.entry(idx))
            .and_then(|entry| entry.last_played_at);
        let updated = WavEntry {
            relative_path: ctx.entry.relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped,
            sound_type: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .and_then(|entry| entry.sound_type),
            locked: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .map(|entry| entry.locked)
                .unwrap_or(false),
            missing: false,
            last_played_at,
            user_tag: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .and_then(|entry| entry.user_tag.clone()),
            tag_named: false,
            normal_tags: ctx.entry.normal_tags.clone(),
        };

        let is_currently_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == ctx.source.id && audio.relative_path == ctx.entry.relative_path
            });
        if is_currently_loaded && was_playing {
            let start_override = if playhead_position.is_finite() {
                Some(f64::from(playhead_position.clamp(0.0, 1.0)))
            } else {
                None
            };
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: ctx.source.id.clone(),
                    relative_path: ctx.entry.relative_path.clone(),
                    looped: was_looping,
                    start_override,
                    force_loaded_audio: false,
                }));
        }

        self.update_cached_entry(&ctx.source, &ctx.entry.relative_path, updated);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&ctx.source.id) {
            self.rebuild_browser_lists();
        }
        self.refresh_waveform_for_sample(&ctx.source, &ctx.entry.relative_path);
        self.set_status(
            format!("Normalized {}", ctx.entry.relative_path.display()),
            StatusTone::Info,
        );
        backup.capture_after(&ctx.absolute_path)?;
        let after = self.capture_meaningful_ui_snapshot();
        let entry = self.selection_edit_undo_entry(
            format!("Normalized {}", ctx.entry.relative_path.display()),
            ctx.source.id.clone(),
            ctx.entry.relative_path.clone(),
            ctx.absolute_path.clone(),
            backup,
        );
        self.push_undo_entry(AppController::attach_meaningful_ui_restore(
            entry, before, after,
        ));
        Ok(())
    }
}
