use super::*;

impl AppController {
    /// Request a destructive edit, showing a confirmation unless yolo mode is enabled.
    pub(crate) fn request_destructive_selection_edit(
        &mut self,
        edit: DestructiveSelectionEdit,
    ) -> Result<SelectionEditRequest, String> {
        if let Err(err) = self.validate_destructive_edit_request(edit) {
            self.set_status(err.clone(), StatusTone::Error);
            return Err(err);
        }
        if self.settings.controls.destructive_yolo_mode {
            self.ui.waveform.pending_destructive = None;
            self.apply_selection_edit_kind(edit)?;
            return Ok(SelectionEditRequest::Applied);
        }
        self.ui.waveform.pending_destructive = Some(prompt::prompt_for_edit(
            edit,
            &self.settings.audio_write_format,
        ));
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
            DestructiveSelectionEdit::CommitEditSelectionFades => {
                self.commit_edit_selection_fades().map(|_| ())
            }
            DestructiveSelectionEdit::CleanExactDuplicateBeats => {
                self.clean_exact_duplicate_beats()
            }
        }
    }

    fn validate_destructive_edit_request(
        &self,
        edit: DestructiveSelectionEdit,
    ) -> Result<(), String> {
        match edit {
            DestructiveSelectionEdit::CleanExactDuplicateBeats => {
                self.validate_duplicate_cleanup_request()
            }
            DestructiveSelectionEdit::CommitEditSelectionFades => {
                self.validate_commit_edit_selection_fades()
            }
            _ => self.validate_selection_edit_target(),
        }
    }

    fn validate_duplicate_cleanup_request(&self) -> Result<(), String> {
        if let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() {
            let source = self
                .library
                .sources
                .iter()
                .find(|source| source.id == audio.source_id)
                .ok_or_else(|| "Source not available for loaded sample".to_string())?;
            let absolute_path = source.root.join(&audio.relative_path);
            crate::app::controller::library::wav_io::ensure_mono_stereo_wav_destructive_edit_target(
                &absolute_path,
                "This edit",
            )?;
        }
        self.exact_duplicate_cleanup_ranges().map(|_| ())
    }

    fn validate_commit_edit_selection_fades(&self) -> Result<(), String> {
        self.validate_selection_edit_target()?;
        let Some(selection) = self.ui.waveform.edit_selection else {
            return Err("Set an edit selection before applying it".to_string());
        };
        if !selection.has_edit_effects() {
            return Err("Adjust an edit fade or gain before applying it".to_string());
        }
        Ok(())
    }

    fn validate_selection_edit_target(&self) -> Result<(), String> {
        let target = self.selection_target()?;
        crate::app::controller::library::wav_io::ensure_mono_stereo_wav_destructive_edit_target(
            &target.absolute_path,
            "This edit",
        )
        .map(|_| ())
    }
}
