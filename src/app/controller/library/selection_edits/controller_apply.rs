use super::*;

pub(super) struct SelectionEditVisualState {
    pub(super) view: WaveformView,
    pub(super) selection: Option<SelectionRange>,
    pub(super) edit_selection: Option<SelectionRange>,
    pub(super) cursor: Option<f32>,
    pub(super) loop_enabled: bool,
}

pub(super) struct PlaybackResumeState {
    pub(super) was_playing: bool,
    pub(super) was_looping: bool,
    pub(super) start_override: Option<f64>,
}

pub(super) struct SelectionEditSession {
    pub(super) target: SelectionTarget,
    pub(super) db: std::rc::Rc<SourceDatabase>,
    pub(super) backup: undo::OverwriteBackup,
    pub(super) tag: Rating,
    pub(super) last_played_at: Option<i64>,
    pub(super) looped: bool,
    pub(super) visual: SelectionEditVisualState,
    pub(super) playback: PlaybackResumeState,
}

pub(super) struct CropNewSampleSession {
    pub(super) target: SelectionTarget,
    pub(super) db: std::rc::Rc<SourceDatabase>,
    pub(super) tag: Rating,
    pub(super) playback: PlaybackResumeState,
}

impl AppController {
    pub(super) fn apply_selection_edit<F>(
        &mut self,
        action_label: &str,
        preserve_selection: bool,
        edit: F,
    ) -> Result<(), String>
    where
        F: FnMut(&mut SelectionEditBuffer) -> Result<(), String>,
    {
        let session = self.begin_selection_edit_session()?;
        let outcome = apply_selection_edit_write(
            SelectionEditWriteRequest {
                target: &session.target,
                db: &session.db,
                tag: session.tag,
                last_played_at: session.last_played_at,
                looped: session.looped,
            },
            edit,
        )?;
        session
            .backup
            .capture_after(&session.target.absolute_path)?;
        self.finish_selection_edit(session, action_label, preserve_selection, outcome.entry)
    }

    pub(super) fn selection_target(&self) -> Result<SelectionTarget, String> {
        let selection =
            selection_target_range(self.ui.waveform.edit_selection, self.ui.waveform.selection);
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample to edit it".to_string())?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == audio.source_id)
            .cloned()
            .ok_or_else(|| "Source not available for loaded sample".to_string())?;
        let relative_path = audio.relative_path.clone();
        let absolute_path = source.root.join(&relative_path);
        Ok(SelectionTarget {
            source,
            relative_path,
            absolute_path,
            selection,
        })
    }

    pub(super) fn begin_selection_edit_session(&mut self) -> Result<SelectionEditSession, String> {
        let target = self.selection_target()?;
        let backup = undo::OverwriteBackup::capture_before(&target.absolute_path)?;
        let db = self
            .database_for(&target.source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let tag = self.sample_tag_for(&target.source, &target.relative_path)?;
        let last_played_at = self.sample_last_played_for(&target.source, &target.relative_path)?;
        let looped = self.sample_looped_for(&target.source, &target.relative_path)?;
        Ok(SelectionEditSession {
            target,
            db,
            backup,
            tag,
            last_played_at,
            looped,
            visual: self.capture_selection_edit_visual_state(),
            playback: self.capture_playback_resume_state(),
        })
    }

    pub(super) fn begin_crop_new_sample_session(&mut self) -> Result<CropNewSampleSession, String> {
        let target = self.selection_target()?;
        let db = self
            .database_for(&target.source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let tag = self.sample_tag_for(&target.source, &target.relative_path)?;
        Ok(CropNewSampleSession {
            target,
            db,
            tag,
            playback: self.capture_playback_resume_state(),
        })
    }

    pub(super) fn finish_selection_edit(
        &mut self,
        session: SelectionEditSession,
        action_label: &str,
        preserve_selection: bool,
        entry: WavEntry,
    ) -> Result<(), String> {
        self.update_cached_entry(&session.target.source, &session.target.relative_path, entry);
        self.clear_loaded_waveform_after_disk_edit();
        self.refresh_waveform_for_sample(&session.target.source, &session.target.relative_path);
        self.restore_selection_edit_visuals(preserve_selection, session.visual);
        self.queue_selection_edit_playback(&session.target, &session.playback);
        self.maybe_trigger_pending_playback();
        self.push_undo_entry(self.selection_edit_undo_entry(
            format!("{action_label} {}", session.target.relative_path.display()),
            session.target.source.id.clone(),
            session.target.relative_path.clone(),
            session.target.absolute_path.clone(),
            session.backup,
        ));
        self.set_status(
            format!(
                "{} {}",
                action_label,
                session.target.relative_path.display()
            ),
            StatusTone::Info,
        );
        Ok(())
    }

    pub(super) fn clear_loaded_waveform_after_disk_edit(&mut self) {
        self.sample_view.wav.loaded_wav = None;
        self.set_ui_loaded_wav(None);
    }

    pub(super) fn capture_selection_edit_visual_state(&self) -> SelectionEditVisualState {
        SelectionEditVisualState {
            view: self.ui.waveform.view,
            selection: self.ui.waveform.selection,
            edit_selection: self.ui.waveform.edit_selection,
            cursor: self.ui.waveform.cursor,
            loop_enabled: self.ui.waveform.loop_enabled,
        }
    }

    pub(super) fn capture_playback_resume_state(&self) -> PlaybackResumeState {
        let was_playing = self.is_playing();
        let was_looping = if self.ui.waveform.loop_enabled {
            true
        } else if self.audio.pending_loop_disable_at.is_some() {
            false
        } else {
            self.audio
                .player
                .as_ref()
                .is_some_and(|player| player.borrow().is_looping())
        };
        let start_override = self
            .ui
            .waveform
            .playhead
            .position
            .is_finite()
            .then(|| f64::from(self.ui.waveform.playhead.position.clamp(0.0, 1.0)));
        PlaybackResumeState {
            was_playing,
            was_looping,
            start_override,
        }
    }

    pub(super) fn restore_selection_edit_visuals(
        &mut self,
        preserve_selection: bool,
        visual: SelectionEditVisualState,
    ) {
        if preserve_selection {
            self.ui.waveform.view = visual.view.clamp();
            self.ui.waveform.cursor = visual.cursor;
            self.ui.waveform.loop_enabled = visual.loop_enabled;
            self.selection_state.range.set_range(visual.selection);
            self.apply_selection(visual.selection);
            self.selection_state
                .edit_range
                .set_range(visual.edit_selection);
            self.apply_edit_selection(visual.edit_selection);
            return;
        }
        self.clear_waveform_selection();
        self.clear_edit_selection();
    }

    pub(super) fn queue_selection_edit_playback(
        &mut self,
        target: &SelectionTarget,
        playback: &PlaybackResumeState,
    ) {
        if !playback.was_playing {
            return;
        }
        self.runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: target.source.id.clone(),
                relative_path: target.relative_path.clone(),
                looped: playback.was_looping,
                start_override: playback.start_override,
            }));
    }
}
