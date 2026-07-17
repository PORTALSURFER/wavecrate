use std::{path::Path, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingWaveformDestructiveEdit, WaveformDestructiveEditKind,
    sample_path_label,
};
use crate::native_app::sample_library::committed_file_mutations::{
    FileMutationChange, FileMutationOperation,
};
use crate::native_app::sample_library::folder_browser::BrowserListingRevealReason;
use crate::native_app::waveform::{WaveformPreservedMarks, WaveformState};

use super::worker::{AppliedWaveformEdit, WaveformDestructiveEditResult};
impl NativeAppState {
    pub(in crate::native_app) fn finish_waveform_destructive_edit(
        &mut self,
        completion: ui::TaskCompletion<WaveformDestructiveEditResult>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(output) = self
            .background
            .waveform_destructive_edit_task
            .finish_completion(completion)
        else {
            return;
        };
        let Some(active) = self.background.waveform_destructive_edit_context.take() else {
            return;
        };
        let applied = match output.result {
            Ok(applied) => applied,
            Err(error) => {
                self.record_failed_file_mutation(
                    FileMutationOperation::Edit,
                    Some(active.request.source.id.as_str().to_string()),
                    error.clone(),
                    context,
                );
                self.ui.status.sample = format!(
                    "{} failed: {error}",
                    active.request.prompt.edit.action_label()
                );
                return;
            }
        };
        if let Some(mark) = output.extracted_mark {
            self.waveform
                .current
                .mark_extracted_play_selection(&mark.source_path, mark.selection);
            self.waveform.current.flash_play_selection();
        }
        if let Some((source_path, operation)) = active.harvest_whole_file_derivation.as_ref() {
            self.record_harvest_whole_file_derivation(
                source_path,
                &applied.absolute_path,
                operation.clone(),
            );
        } else {
            self.mark_harvest_touched_for_path(&active.request.absolute_path);
        }
        let extracted_metadata_error = if let Some(extracted_path) = applied
            .extracted
            .as_ref()
            .map(|extracted| extracted.path.clone())
            && let Err(error) = self.assign_extracted_file_metadata(
                &extracted_path,
                active.extracted_playback_type,
                context,
            ) {
            Some(error)
        } else {
            None
        };
        if let Some(extracted) = applied.extracted.as_ref() {
            self.record_harvest_extraction_with_source_duration(
                &active.request.absolute_path,
                active.request.selection,
                &extracted.path,
                active.source_duration_seconds.unwrap_or_default(),
            );
        }
        let primary_change = if active.harvest_whole_file_derivation.is_some() {
            FileMutationChange::created(applied.absolute_path.clone())
        } else {
            FileMutationChange::content_changed(applied.absolute_path.clone())
                .with_before_content_identity(applied.before_content_identity.clone())
        };
        let mut mutation_changes = vec![primary_change];
        if let Some(extracted) = applied.extracted.as_ref() {
            mutation_changes.push(FileMutationChange::created(extracted.path.clone()));
        }
        self.queue_partially_committed_file_mutation(
            FileMutationOperation::Edit,
            mutation_changes,
            extracted_metadata_error
                .iter()
                .cloned()
                .map(|error| (None, error))
                .collect(),
            context,
        );
        let visual_error = self
            .apply_destructive_edit_visual_state(
                &applied,
                active.before_selected_path.as_deref(),
                &active.request,
                active.preserved_marks,
            )
            .err();
        if let Some(output_path) = active.output_focus_path.as_ref() {
            self.library
                .folder_browser
                .refresh_file_path_across_sources(output_path);
            self.library
                .folder_browser
                .focus_file_across_sources_matching_tags_for_reason(
                    output_path,
                    &self.metadata.tags_by_file,
                    BrowserListingRevealReason::DestructiveEditReload,
                );
            self.load_navigation_sample_validated(
                output_path.to_string_lossy().to_string(),
                context,
                Instant::now(),
            );
        }
        self.register_destructive_edit_transaction(active.request.prompt.edit, applied);

        let label = sample_path_label(&active.request.absolute_path);
        self.ui.status.sample = if let Some(error) = visual_error.as_ref() {
            format!(
                "{} {label}; edit committed but waveform reload failed: {error}",
                active.request.prompt.edit.past_tense_label()
            )
        } else if let Some(error) = extracted_metadata_error.as_ref() {
            format!(
                "{} {label}; extracted metadata incomplete: {error}",
                active.request.prompt.edit.past_tense_label()
            )
        } else if active.playback_was_active {
            format!(
                "{} {label} and stopped playback",
                active.request.prompt.edit.past_tense_label()
            )
        } else {
            format!("{} {label}", active.request.prompt.edit.past_tense_label())
        };
    }

    fn apply_destructive_edit_visual_state(
        &mut self,
        applied: &AppliedWaveformEdit,
        before_selected_path: Option<&str>,
        request: &PendingWaveformDestructiveEdit,
        preserved_marks: Option<WaveformPreservedMarks>,
    ) -> Result<(), String> {
        self.evict_waveform_cache_path(&applied.absolute_path);
        self.library.folder_browser.refresh_filesystem_paths(
            &applied.source_id,
            std::slice::from_ref(&applied.relative_path),
        );
        if let Some(extracted) = applied.extracted.as_ref() {
            self.library
                .folder_browser
                .refresh_file_path(&extracted.path);
        }
        let edited_path_id = applied.absolute_path.to_string_lossy();
        if before_selected_path.is_none() || before_selected_path == Some(edited_path_id.as_ref()) {
            self.library
                .folder_browser
                .focus_file_across_sources_matching_tags_for_reason(
                    &applied.absolute_path,
                    &self.metadata.tags_by_file,
                    BrowserListingRevealReason::DestructiveEditReload,
                );
        }
        self.reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        if let Some(marks) = preserved_marks
            && self.waveform.current.path() == applied.absolute_path
        {
            self.waveform.current.restore_preserved_marks(marks);
        }
        if matches!(
            request.prompt.edit,
            WaveformDestructiveEditKind::ApplyEditSelectionEffects
        ) && self.waveform.current.path() == applied.absolute_path
        {
            self.waveform
                .current
                .set_edit_selection_range(request.selection.clear_fades().with_gain(1.0));
            self.waveform.current.flash_edit_selection();
        }
        Ok(())
    }

    pub(super) fn preserved_marks_after_destructive_edit(
        &self,
        request: &PendingWaveformDestructiveEdit,
    ) -> Option<WaveformPreservedMarks> {
        if self.waveform.current.path() != request.absolute_path {
            return None;
        }
        match request.prompt.edit {
            WaveformDestructiveEditKind::TrimSelection
            | WaveformDestructiveEditKind::ExtractAndTrimSelection => Some(
                self.waveform
                    .current
                    .preserved_marks_after_trim(request.selection),
            ),
            WaveformDestructiveEditKind::CropSelection => Some(
                self.waveform
                    .current
                    .preserved_marks_after_crop(request.selection),
            ),
            WaveformDestructiveEditKind::ReverseSelection
            | WaveformDestructiveEditKind::MuteSelection
            | WaveformDestructiveEditKind::SlideSampleAudio { .. } => {
                Some(self.waveform.current.preserved_marks_unchanged())
            }
            WaveformDestructiveEditKind::ApplyEditSelectionEffects => None,
        }
    }

    fn register_destructive_edit_transaction(
        &mut self,
        kind: WaveformDestructiveEditKind,
        applied: AppliedWaveformEdit,
    ) {
        let undo_applied = applied.clone();
        let redo_applied = applied;
        self.begin_transaction(kind.transaction_label());
        self.register_transaction_action(
            kind.undo_label(),
            move |transaction| {
                transaction.restore_edited_waveform(&undo_applied.backup.before, &undo_applied)
            },
            move |transaction| {
                transaction.restore_edited_waveform(&redo_applied.backup.after, &redo_applied)
            },
        );
        self.commit_transaction();
    }

    pub(super) fn reload_waveform_path_now_if_loaded(
        &mut self,
        absolute_path: &Path,
    ) -> Result<(), String> {
        if self.waveform.current.path() != absolute_path {
            return Ok(());
        }
        self.waveform.current = WaveformState::load_path_with_progress_and_cancel(
            absolute_path.to_path_buf(),
            |_| {},
            || false,
        )?;
        Ok(())
    }
}
