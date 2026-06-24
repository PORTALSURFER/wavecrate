use std::path::Path;

use radiant::prelude as ui;
use wavecrate::sample_sources::config::AudioWriteFormatConfig;
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{
    ExtractedFilePlaybackType, GuiMessage, NativeAppState, PendingWaveformDestructiveEdit,
    WaveformDestructiveEditKind, WaveformDestructiveEditUiContext, sample_path_label,
};
use crate::native_app::transaction_history::TransactionContext;
use crate::native_app::waveform::{WaveformPreservedMarks, WaveformState};

mod worker;
pub(in crate::native_app) use worker::WaveformDestructiveEditResult;
use worker::{AppliedWaveformEdit, WaveformDestructiveEditWorkerRequest};

const WAVEFORM_DESTRUCTIVE_EDIT_TASK_NAME: &str = "gui-waveform-destructive-edit";

impl NativeAppState {
    pub(in crate::native_app) fn request_crop_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(WaveformDestructiveEditKind::CropSelection, context);
    }

    pub(in crate::native_app) fn request_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(WaveformDestructiveEditKind::TrimSelection, context);
    }

    pub(in crate::native_app) fn request_extract_and_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ExtractAndTrimSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_apply_edit_selection_effects(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ApplyEditSelectionEffects,
            context,
        );
    }

    fn request_waveform_destructive_edit(
        &mut self,
        kind: WaveformDestructiveEditKind,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let request = match self.pending_destructive_edit_request(kind) {
            Ok(request) => request,
            Err(error) => {
                self.ui.status.sample = error;
                return;
            }
        };

        if self.ui.settings.persisted.controls.destructive_yolo_mode {
            self.ui
                .browser_interaction
                .pending_waveform_destructive_edit = None;
            if let Err(error) = self.queue_destructive_edit_request(request, context) {
                self.ui.status.sample = format!("{} failed: {error}", kind.action_label());
            }
            return;
        }

        self.ui
            .browser_interaction
            .pending_waveform_destructive_edit = Some(request);
    }

    pub(in crate::native_app) fn confirm_pending_waveform_destructive_edit(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(request) = self
            .ui
            .browser_interaction
            .pending_waveform_destructive_edit
            .take()
        else {
            return;
        };

        let result = self.queue_destructive_edit_request(request, context);
        if let Err(error) = result {
            self.ui.status.sample = format!("Edit failed: {error}");
        }
    }

    pub(in crate::native_app) fn cancel_pending_waveform_destructive_edit(&mut self) {
        self.ui
            .browser_interaction
            .pending_waveform_destructive_edit = None;
    }

    fn pending_destructive_edit_request(
        &self,
        kind: WaveformDestructiveEditKind,
    ) -> Result<PendingWaveformDestructiveEdit, String> {
        let absolute_path = self.waveform.current.path();
        if !self.waveform.current.has_loaded_sample() || absolute_path.as_os_str().is_empty() {
            return Err(format!("Load a sample before {}", kind.gerund_label()));
        }
        let selection = self.destructive_edit_selection_for_kind(kind)?;
        let (source, relative_path) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&absolute_path)
            .ok_or_else(|| String::from("Loaded sample is not inside a configured source"))?;
        if let Some(error) = self
            .library
            .folder_browser
            .file_change_lock_error(&absolute_path, kind.action_label())
        {
            return Err(error);
        }
        Ok(PendingWaveformDestructiveEdit {
            prompt: destructive_edit_prompt(kind, &self.ui.settings.persisted.audio_write_format),
            source,
            relative_path,
            absolute_path,
            selection,
        })
    }

    fn queue_destructive_edit_request(
        &mut self,
        request: PendingWaveformDestructiveEdit,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Result<(), String> {
        if let Some(error) = self
            .library
            .folder_browser
            .file_change_lock_error(&request.absolute_path, request.prompt.edit.action_label())
        {
            return Err(error);
        }
        let extraction_request =
            if request.prompt.edit == WaveformDestructiveEditKind::ExtractAndTrimSelection {
                Some(
                    self.waveform
                        .current
                        .selection_extraction_request(None, request.selection)?,
                )
            } else {
                None
            };
        let before_selected_path = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let playback_was_active = self.waveform.current.is_playing();
        let extracted_playback_type =
            ExtractedFilePlaybackType::from_loop_active(self.audio.loop_playback);
        self.stop_audio_output_playback();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.pending_playback_start = None;
        self.audio.pending_sample_playback = None;
        self.audio.pending_runtime_start = None;

        let preserved_marks = self.preserved_marks_after_destructive_edit(&request);
        let worker_request =
            WaveformDestructiveEditWorkerRequest::new(request.clone(), extraction_request);
        self.background.waveform_destructive_edit_context =
            Some(WaveformDestructiveEditUiContext {
                request: request.clone(),
                before_selected_path,
                playback_was_active,
                extracted_playback_type,
                preserved_marks,
            });
        self.ui.status.sample = format!(
            "{} {}",
            request.prompt.edit.action_label(),
            sample_path_label(&request.absolute_path)
        );
        context
            .business()
            .blocking_io(WAVEFORM_DESTRUCTIVE_EDIT_TASK_NAME)
            .latest(&mut self.background.waveform_destructive_edit_task)
            .run(
                move |_| worker::execute_destructive_edit(worker_request),
                GuiMessage::WaveformDestructiveEditFinished,
            );
        Ok(())
    }

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
        if let Err(error) = self.apply_destructive_edit_visual_state(
            &applied,
            active.before_selected_path.as_deref(),
            &active.request,
            active.preserved_marks,
        ) {
            self.ui.status.sample = format!(
                "{} failed: {error}",
                active.request.prompt.edit.action_label()
            );
            return;
        }
        let playback_type_tag_error = if let Some(extracted_path) = applied
            .extracted
            .as_ref()
            .map(|extracted| extracted.path.clone())
            && let Err(error) = self.tag_extracted_file_playback_type(
                &extracted_path,
                active.extracted_playback_type,
                context,
            ) {
            Some(error)
        } else {
            None
        };
        self.register_destructive_edit_transaction(active.request.prompt.edit, applied);

        let label = sample_path_label(&active.request.absolute_path);
        self.ui.status.sample = if let Some(error) = playback_type_tag_error {
            format!(
                "{} {label}; playback type tag not saved: {error}",
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
        if before_selected_path == Some(applied.absolute_path.to_string_lossy().as_ref()) {
            self.library
                .folder_browser
                .select_file(applied.absolute_path.display().to_string());
        }
        self.reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        if let Some(marks) = preserved_marks
            && self.waveform.current.path() == applied.absolute_path
        {
            self.waveform.current.restore_preserved_marks(marks);
        }
        if request.prompt.edit == WaveformDestructiveEditKind::ApplyEditSelectionEffects
            && self.waveform.current.path() == applied.absolute_path
        {
            self.waveform
                .current
                .set_edit_selection_range(request.selection.clear_fades().with_gain(1.0));
            self.waveform.current.flash_edit_selection();
        }
        Ok(())
    }

    fn preserved_marks_after_destructive_edit(
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
            WaveformDestructiveEditKind::ApplyEditSelectionEffects => None,
        }
    }

    fn destructive_edit_selection_for_kind(
        &self,
        kind: WaveformDestructiveEditKind,
    ) -> Result<SelectionRange, String> {
        if kind == WaveformDestructiveEditKind::ApplyEditSelectionEffects {
            let selection = self
                .waveform
                .current
                .edit_selection()
                .filter(|selection| selection.width() > 0.0)
                .ok_or_else(|| String::from("Set an edit selection before applying it"))?;
            if !selection.has_edit_effects() {
                return Err(String::from(
                    "Adjust an edit fade or gain before applying it",
                ));
            }
            return Ok(selection);
        }
        self.waveform
            .current
            .destructive_edit_selection()
            .ok_or_else(|| format!("Mark an edit or play range before {}", kind.gerund_label()))
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

    fn reload_waveform_path_now_if_loaded(&mut self, absolute_path: &Path) -> Result<(), String> {
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

impl TransactionContext<'_> {
    fn restore_edited_waveform(
        &mut self,
        backup_path: &Path,
        applied: &AppliedWaveformEdit,
    ) -> Result<(), String> {
        if let Some(error) = self
            .state
            .library
            .folder_browser
            .file_change_lock_error(&applied.absolute_path, "Undo")
        {
            return Err(error);
        }
        worker::restore_edited_waveform(backup_path, applied)?;
        self.state.evict_waveform_cache_path(&applied.absolute_path);
        let mut relative_paths = vec![applied.relative_path.clone()];
        if let Some(extracted) = applied.extracted.as_ref() {
            relative_paths.push(extracted.relative_path.clone());
        }
        self.state
            .library
            .folder_browser
            .refresh_filesystem_paths(&applied.source_id, &relative_paths);
        self.state
            .reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        Ok(())
    }
}

impl WaveformDestructiveEditKind {
    fn action_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Crop",
            Self::TrimSelection => "Trim",
            Self::ExtractAndTrimSelection => "Extract and trim",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
        }
    }

    fn gerund_label(self) -> &'static str {
        match self {
            Self::CropSelection => "cropping",
            Self::TrimSelection => "trimming",
            Self::ExtractAndTrimSelection => "extracting and trimming",
            Self::ApplyEditSelectionEffects => "applying edit mark edits",
        }
    }

    fn past_tense_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Cropped",
            Self::TrimSelection => "Trimmed",
            Self::ExtractAndTrimSelection => "Extracted and trimmed",
            Self::ApplyEditSelectionEffects => "Applied edit mark edits to",
        }
    }

    fn transaction_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Crop waveform selection",
            Self::TrimSelection => "Trim waveform selection",
            Self::ExtractAndTrimSelection => "Extract and trim waveform selection",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
        }
    }

    fn undo_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Restore cropped audio",
            Self::TrimSelection => "Restore trimmed audio",
            Self::ExtractAndTrimSelection => "Restore extracted and trimmed audio",
            Self::ApplyEditSelectionEffects => "Restore edit mark edits",
        }
    }
}

fn destructive_edit_prompt(
    edit: WaveformDestructiveEditKind,
    write_format: &AudioWriteFormatConfig,
) -> crate::native_app::app::WaveformDestructiveEditPrompt {
    let message = match edit {
        WaveformDestructiveEditKind::CropSelection => {
            "This will keep only the selected region and remove audio outside it from the source file."
        }
        WaveformDestructiveEditKind::TrimSelection => {
            "This will remove the selected region and close the gap in the source file."
        }
        WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            "This will extract the selected region into a new sibling file, then remove that region and close the gap in the source file."
        }
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            "This will overwrite the edit selection with the currently previewed fade and gain edits."
        }
    };
    crate::native_app::app::WaveformDestructiveEditPrompt {
        edit,
        title: destructive_edit_title(edit),
        message: format!(
            "{message} Wavecrate will rewrite the file using the current write format: {}.",
            write_format.summary_label()
        ),
    }
}

fn destructive_edit_title(edit: WaveformDestructiveEditKind) -> String {
    match edit {
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            String::from("Apply edit mark edits")
        }
        _ => format!("{} selection", edit.action_label()),
    }
}
