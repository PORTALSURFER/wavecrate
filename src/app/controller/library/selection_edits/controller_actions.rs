use super::*;

impl AppController {
    /// Request a destructive edit, showing a confirmation unless yolo mode is enabled.
    pub(crate) fn request_destructive_selection_edit(
        &mut self,
        edit: DestructiveSelectionEdit,
    ) -> Result<SelectionEditRequest, String> {
        if let Err(err) = self.selection_target() {
            self.set_status(err.clone(), StatusTone::Error);
            return Err(err);
        }
        if self.settings.controls.destructive_yolo_mode {
            self.ui.waveform.pending_destructive = None;
            self.apply_selection_edit_kind(edit)?;
            return Ok(SelectionEditRequest::Applied);
        }
        self.ui.waveform.pending_destructive = Some(prompt::prompt_for_edit(edit));
        Ok(SelectionEditRequest::Prompted)
    }

    /// Apply the pending destructive edit after user confirmation.
    pub(crate) fn apply_confirmed_destructive_edit(&mut self, edit: DestructiveSelectionEdit) {
        self.ui.waveform.pending_destructive = None;
        let _ = self.apply_selection_edit_kind(edit);
    }

    /// Clear any pending destructive edit prompt without applying it.
    pub(crate) fn clear_destructive_prompt(&mut self) {
        self.ui.waveform.pending_destructive = None;
    }

    pub(crate) fn has_pending_destructive_prompt(&self) -> bool {
        self.ui.waveform.pending_destructive.is_some()
    }

    pub(crate) fn apply_pending_destructive_prompt(&mut self) -> bool {
        let Some(prompt) = self.ui.waveform.pending_destructive.clone() else {
            return false;
        };
        self.apply_confirmed_destructive_edit(prompt.edit);
        true
    }

    /// Apply edit-selection fades to disk and clear preview fades.
    pub(crate) fn commit_edit_selection_fades(&mut self) -> Result<bool, String> {
        let Some(selection) = self.ui.waveform.edit_selection else {
            return Ok(false);
        };
        if !selection.has_edit_effects() {
            return Ok(false);
        }
        let result = self.apply_selection_edit("Applied edit fades", true, |buffer| {
            apply_selection_fades(SelectionFadeRequest {
                samples: &mut buffer.samples,
                channels: buffer.channels,
                sample_rate: buffer.sample_rate,
                start_frame: buffer.start_frame,
                end_frame: buffer.end_frame,
                selection_gain: selection.gain(),
                fade_in: selection.fade_in(),
                fade_out: selection.fade_out(),
            });
            Ok(())
        });
        match result {
            Ok(()) => {
                let cleared = selection.clear_fades().with_gain(1.0);
                self.selection_state.edit_range.set_range(Some(cleared));
                self.apply_edit_selection(Some(cleared));
                Ok(true)
            }
            Err(err) => {
                self.set_status(err.clone(), StatusTone::Error);
                Err(err)
            }
        }
    }

    /// Cancel preview fades for the edit selection without writing audio.
    pub(crate) fn cancel_edit_selection_fades(&mut self) -> bool {
        let Some(selection) = self.ui.waveform.edit_selection else {
            return false;
        };
        if !selection.has_edit_effects() {
            return false;
        }
        let cleared = selection.clear_fades().with_gain(1.0);
        self.selection_state.edit_range.set_range(Some(cleared));
        self.apply_edit_selection(Some(cleared));
        true
    }

    /// Crop the loaded sample to the active selection range and refresh caches/exports.
    pub(crate) fn crop_waveform_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Cropped selection", false, crop_buffer);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Write the cropped selection to a new sample file alongside the original.
    pub(crate) fn crop_waveform_selection_to_new_sample(&mut self) -> Result<(), String> {
        let session = self.begin_crop_new_sample_session()?;
        let new_relative = buffer::next_crop_relative_path(
            &session.target.relative_path,
            &session.target.source.root,
        )?;
        let edge_fade = self
            .settings
            .controls
            .auto_edge_fades_on_selection_exports
            .then(|| {
                Duration::from_secs_f32(self.settings.controls.anti_clip_fade_ms.max(0.0) / 1000.0)
            });
        let outcome = crop_selection_to_new_sample_write(CropNewSampleWriteRequest {
            target: &session.target,
            new_relative,
            db: &session.db,
            tag: session.tag,
            edge_fade,
        })?;

        self.insert_cached_entry(&session.target.source, outcome.entry.clone());
        self.enqueue_similarity_for_new_sample(
            &session.target.source,
            &outcome.entry.relative_path,
            outcome.entry.file_size,
            outcome.entry.modified_ns,
        );
        self.refresh_waveform_for_sample(&session.target.source, &session.target.relative_path);

        if let Ok(backup) = undo::OverwriteBackup::capture_before(&outcome.new_absolute)
            && backup.capture_after(&outcome.new_absolute).is_ok()
        {
            self.push_undo_entry(self.crop_new_sample_undo_entry(
                format!(
                    "Cropped to new sample {}",
                    outcome.entry.relative_path.display()
                ),
                session.target.source.id.clone(),
                outcome.entry.relative_path.clone(),
                outcome.new_absolute.clone(),
                session.tag,
                backup,
            ));
        }

        if session.playback.was_playing {
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: session.target.source.id.clone(),
                    relative_path: outcome.entry.relative_path.clone(),
                    looped: session.playback.was_looping,
                    start_override: session.playback.start_override,
                }));
        }

        let _ =
            self.load_waveform_for_selection(&session.target.source, &outcome.entry.relative_path);
        self.focus_waveform();
        self.set_status(
            format!(
                "Cropped to new sample {}",
                outcome.entry.relative_path.display()
            ),
            StatusTone::Info,
        );
        Ok(())
    }

    /// Remove the selected span from the loaded sample.
    pub(crate) fn trim_waveform_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Trimmed selection", false, trim_buffer);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Fade the selected span down to silence using the given direction.
    pub(crate) fn fade_waveform_selection(
        &mut self,
        direction: FadeDirection,
    ) -> Result<(), String> {
        let result = self.apply_selection_edit("Applied fade", true, |buffer| {
            apply_directional_fade(
                &mut buffer.samples,
                buffer.channels,
                buffer.start_frame,
                buffer.end_frame,
                direction,
            );
            Ok(())
        });
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Normalize the active selection and apply short fades at the edges.
    pub(crate) fn normalize_waveform_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Normalized selection", true, |buffer| {
            normalize_selection(buffer, Duration::from_millis(5))
        });
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Apply short fade-in/out ramps at the selection edges to reduce clicks.
    pub(crate) fn soften_waveform_selection_edges(&mut self) -> Result<(), String> {
        let fade_ms = self.ui.controls.anti_clip_fade_ms.max(0.0);
        let fade_duration = Duration::from_secs_f32(fade_ms / 1000.0);
        let result = self.apply_selection_edit("Applied short fades", true, |buffer| {
            let selection_frames = buffer.end_frame.saturating_sub(buffer.start_frame);
            let fade_frames =
                edge_fade_frame_count(buffer.sample_rate.max(1), selection_frames, fade_duration);
            if fade_frames == 0 {
                return Err("Selection is too short for edge fades".into());
            }
            apply_edge_fades(
                &mut buffer.samples,
                buffer.channels,
                buffer.start_frame,
                buffer.end_frame,
                fade_frames,
            );
            Ok(())
        });
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Repair clicks inside the selection by interpolating the span.
    pub(crate) fn repair_clicks_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Removed clicks", true, |buffer| {
            repair_clicks_buffer(buffer)
        });
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Silence the selected span without applying fades.
    pub(crate) fn mute_waveform_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Muted selection", true, ops::mute_buffer);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Reverse the selected span in time.
    pub(crate) fn reverse_waveform_selection(&mut self) -> Result<(), String> {
        let result = self.apply_selection_edit("Reversed selection", true, reverse_buffer);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    fn apply_selection_edit_kind(&mut self, edit: DestructiveSelectionEdit) -> Result<(), String> {
        match edit {
            DestructiveSelectionEdit::CropSelection => self.crop_waveform_selection(),
            DestructiveSelectionEdit::TrimSelection => self.trim_waveform_selection(),
            DestructiveSelectionEdit::ReverseSelection => self.reverse_waveform_selection(),
            DestructiveSelectionEdit::FadeLeftToRight => {
                self.fade_waveform_selection(FadeDirection::LeftToRight)
            }
            DestructiveSelectionEdit::FadeRightToLeft => {
                self.fade_waveform_selection(FadeDirection::RightToLeft)
            }
            DestructiveSelectionEdit::ShortEdgeFades => self.soften_waveform_selection_edges(),
            DestructiveSelectionEdit::MuteSelection => self.mute_waveform_selection(),
            DestructiveSelectionEdit::NormalizeSelection => self.normalize_waveform_selection(),
            DestructiveSelectionEdit::ClickRemoval => self.repair_clicks_selection(),
        }
    }
}
