use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{HarvestDerivationOperation, config::AudioWriteFormatConfig};
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{
    ExtractedFilePlaybackType, GuiMessage, NativeAppState, PendingWaveformDestructiveEdit,
    WaveformDestructiveEditKind, WaveformDestructiveEditUiContext, sample_path_label,
};
use crate::native_app::sample_library::folder_browser::BrowserListingRevealReason;
use crate::native_app::transaction_history::TransactionContext;
use crate::native_app::waveform::{
    WaveformPreservedMarks, WaveformSelectionKind, WaveformState, execute_waveform_extraction,
};

mod worker;
pub(in crate::native_app) use worker::WaveformDestructiveEditResult;
use worker::{AppliedWaveformEdit, WaveformDestructiveEditWorkerRequest};

const WAVEFORM_DESTRUCTIVE_EDIT_TASK_NAME: &str = "gui-waveform-destructive-edit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaveformDestructiveEditTarget {
    ActiveSelection,
    PlaySelection,
}

#[derive(Default)]
struct WaveformDestructiveEditQueueOptions {
    copy_source_path: Option<PathBuf>,
    output_focus_path: Option<PathBuf>,
    harvest_whole_file_derivation: Option<(PathBuf, HarvestDerivationOperation)>,
}

impl WaveformDestructiveEditTarget {
    fn missing_selection_message(self, kind: WaveformDestructiveEditKind) -> String {
        match self {
            Self::ActiveSelection => {
                format!("Mark an edit or play range before {}", kind.gerund_label())
            }
            Self::PlaySelection => format!("Mark a play range before {}", kind.gerund_label()),
        }
    }

    fn fallback_selection_kind(self) -> WaveformSelectionKind {
        match self {
            Self::ActiveSelection => WaveformSelectionKind::Edit,
            Self::PlaySelection => WaveformSelectionKind::Play,
        }
    }
}

fn waveform_action_denied_error(error: &str) -> bool {
    error.contains("This source is protected") || error.contains("blocked by locked folder")
}

fn harvest_region_copy_operation(
    kind: WaveformDestructiveEditKind,
) -> Option<HarvestDerivationOperation> {
    match kind {
        WaveformDestructiveEditKind::CropSelection => Some(HarvestDerivationOperation::CropCopy),
        WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            Some(HarvestDerivationOperation::Extract)
        }
        WaveformDestructiveEditKind::TrimSelection
        | WaveformDestructiveEditKind::ReverseSelection
        | WaveformDestructiveEditKind::MuteSelection
        | WaveformDestructiveEditKind::ApplyEditSelectionEffects
        | WaveformDestructiveEditKind::SlideSampleAudio { .. } => None,
    }
}

fn harvest_whole_file_copy_operation(
    kind: WaveformDestructiveEditKind,
) -> Option<HarvestDerivationOperation> {
    match kind {
        WaveformDestructiveEditKind::TrimSelection => Some(HarvestDerivationOperation::TrimCopy),
        WaveformDestructiveEditKind::ReverseSelection => {
            Some(HarvestDerivationOperation::ReverseCopy)
        }
        WaveformDestructiveEditKind::MuteSelection => Some(HarvestDerivationOperation::EditCopy),
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            Some(HarvestDerivationOperation::EditCopy)
        }
        WaveformDestructiveEditKind::SlideSampleAudio { .. } => {
            Some(HarvestDerivationOperation::SlideCopy)
        }
        WaveformDestructiveEditKind::CropSelection
        | WaveformDestructiveEditKind::ExtractAndTrimSelection => None,
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn request_crop_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::CropSelection,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::TrimSelection,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_reverse_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ReverseSelection,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_mute_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::MuteSelection,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_extract_and_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ExtractAndTrimSelection,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_crop_playmark_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::CropSelection,
            WaveformDestructiveEditTarget::PlaySelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_trim_playmark_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::TrimSelection,
            WaveformDestructiveEditTarget::PlaySelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_reverse_playmark_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ReverseSelection,
            WaveformDestructiveEditTarget::PlaySelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_extract_and_trim_playmark_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ExtractAndTrimSelection,
            WaveformDestructiveEditTarget::PlaySelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_apply_edit_selection_effects(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ApplyEditSelectionEffects,
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_slide_loaded_sample_audio(
        &mut self,
        frame_offset: i64,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if frame_offset == 0 {
            self.ui.status.sample = String::from("Sample slide cancelled");
            return;
        }
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::SlideSampleAudio { frame_offset },
            WaveformDestructiveEditTarget::ActiveSelection,
            context,
        );
    }

    fn request_waveform_destructive_edit(
        &mut self,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Some(harvest_operation) = harvest_whole_file_copy_operation(kind) {
            match self.queue_harvest_whole_file_copy_edit_request(
                kind,
                target,
                harvest_operation,
                context,
            ) {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    self.flash_denied_destructive_selection_for_error(&error, kind, target);
                    self.ui.status.sample = error;
                    return;
                }
            }
        }
        if let Some(harvest_operation) = harvest_region_copy_operation(kind) {
            match self.queue_harvest_region_copy_request(
                kind,
                target,
                harvest_operation,
                started_at,
                context,
            ) {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    self.flash_denied_destructive_selection_for_error(&error, kind, target);
                    self.ui.status.sample = error;
                    return;
                }
            }
        }

        let request = match self.pending_destructive_edit_request(kind, target) {
            Ok(request) => request,
            Err(error) => {
                self.flash_denied_destructive_selection_for_error(&error, kind, target);
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

    fn queue_harvest_region_copy_request(
        &mut self,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
        harvest_operation: HarvestDerivationOperation,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Result<bool, String> {
        let (absolute_path, selection) = self.destructive_edit_target_for_kind(kind, target)?;
        if !self.harvest_copy_edits_enabled_for_path(&absolute_path) {
            return Ok(false);
        }
        let request = self
            .waveform
            .current
            .selection_extraction_request(None, selection)?;
        let request = self.route_harvest_destination_extraction_request(request)?;
        self.validate_waveform_extraction_target(&request)?;
        let playback_type = ExtractedFilePlaybackType::from_loop_active(self.audio.loop_playback);
        self.ui.status.sample = String::from("Extracting selection to harvest destination");
        context.business().background("gui-waveform-extract").run(
            move |_| execute_waveform_extraction(request),
            move |completion| GuiMessage::PlaySelectionExtractionFinished {
                completion,
                drag_position: None,
                playback_type,
                harvest_operation,
                started_at,
            },
        );
        Ok(true)
    }

    fn queue_harvest_whole_file_copy_edit_request(
        &mut self,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
        harvest_operation: HarvestDerivationOperation,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Result<bool, String> {
        let (source_path, selection) = self.destructive_edit_target_for_kind(kind, target)?;
        if !self.harvest_copy_edits_enabled_for_path(&source_path) {
            return Ok(false);
        }
        let target_folder = self.harvest_destination_for_origin(&source_path)?;
        if let Some(error) = self
            .library
            .folder_browser
            .folder_target_lock_error(&target_folder, kind.action_label())
        {
            return Err(error);
        }
        let Some(primary_source) = self.library.folder_browser.primary_sample_source() else {
            return Err(String::from(
                "Set a Primary source before editing to a harvest destination",
            ));
        };
        let child_path = next_copy_edit_path(&source_path, &target_folder, kind)?;
        let relative_path = child_path
            .strip_prefix(&primary_source.root)
            .map(Path::to_path_buf)
            .map_err(|_| String::from("Harvest edit copy is outside the Primary source"))?;
        let request = PendingWaveformDestructiveEdit {
            prompt: destructive_edit_prompt(kind, &self.ui.settings.persisted.audio_write_format),
            source: primary_source,
            relative_path,
            absolute_path: child_path.clone(),
            selection,
        };
        self.queue_destructive_edit_request_with_options(
            request,
            WaveformDestructiveEditQueueOptions {
                copy_source_path: Some(source_path.clone()),
                output_focus_path: Some(child_path),
                harvest_whole_file_derivation: Some((source_path, harvest_operation)),
            },
            context,
        )?;
        Ok(true)
    }

    fn harvest_copy_edits_enabled_for_path(&self, source_path: &Path) -> bool {
        self.library
            .folder_browser
            .sample_source_for_file_path(source_path)
            .is_some_and(|(source, _)| {
                source.is_protected() || self.library.folder_browser.harvest_filter().is_some()
            })
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

        let denied_selection = request.selection;
        let result = self.queue_destructive_edit_request(request, context);
        if let Err(error) = result {
            self.flash_denied_waveform_selection_for_error(
                &error,
                Some(denied_selection),
                WaveformSelectionKind::Edit,
            );
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
        target: WaveformDestructiveEditTarget,
    ) -> Result<PendingWaveformDestructiveEdit, String> {
        let (absolute_path, selection) = self.destructive_edit_target_for_kind(kind, target)?;
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
        self.queue_destructive_edit_request_with_options(
            request,
            WaveformDestructiveEditQueueOptions::default(),
            context,
        )
    }

    fn queue_destructive_edit_request_with_options(
        &mut self,
        request: PendingWaveformDestructiveEdit,
        options: WaveformDestructiveEditQueueOptions,
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
        let source_duration_seconds = (self.waveform.current.path() == request.absolute_path)
            .then(|| self.waveform.current.duration_seconds() as f64);
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
        let mut worker_request =
            WaveformDestructiveEditWorkerRequest::new(request.clone(), extraction_request);
        if let Some(copy_source_path) = options.copy_source_path.clone() {
            worker_request = worker_request.with_copy_source(copy_source_path);
        }
        self.background.waveform_destructive_edit_context =
            Some(WaveformDestructiveEditUiContext {
                request: request.clone(),
                before_selected_path,
                playback_was_active,
                source_duration_seconds,
                extracted_playback_type,
                preserved_marks,
                output_focus_path: options.output_focus_path,
                harvest_whole_file_derivation: options.harvest_whole_file_derivation,
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

    fn flash_denied_destructive_selection_for_error(
        &mut self,
        error: &str,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
    ) {
        let selection = self
            .destructive_edit_target_for_kind(kind, target)
            .ok()
            .map(|(_, selection)| selection);
        self.flash_denied_waveform_selection_for_error(
            error,
            selection,
            target.fallback_selection_kind(),
        );
    }

    pub(in crate::native_app) fn flash_denied_waveform_selection_for_error(
        &mut self,
        error: &str,
        selection: Option<SelectionRange>,
        fallback_kind: WaveformSelectionKind,
    ) {
        if !waveform_action_denied_error(error) {
            return;
        }
        if let Some(selection) = selection {
            self.waveform
                .current
                .flash_denied_selection_matching(selection, fallback_kind);
        } else {
            self.waveform.current.flash_denied_selection(fallback_kind);
        }
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
        self.ui.status.sample = if let Some(error) = extracted_metadata_error {
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
            WaveformDestructiveEditKind::ReverseSelection
            | WaveformDestructiveEditKind::MuteSelection
            | WaveformDestructiveEditKind::SlideSampleAudio { .. } => {
                Some(self.waveform.current.preserved_marks_unchanged())
            }
            WaveformDestructiveEditKind::ApplyEditSelectionEffects => None,
        }
    }

    fn destructive_edit_target_for_kind(
        &self,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
    ) -> Result<(PathBuf, SelectionRange), String> {
        let absolute_path = self.waveform.current.path();
        let has_loaded_waveform =
            self.waveform.current.has_loaded_sample() && !absolute_path.as_os_str().is_empty();

        if has_loaded_waveform {
            if let Some(selection) = self.destructive_edit_selection_for_kind(kind, target)? {
                return Ok((absolute_path, selection));
            }
        } else if !matches!(kind, WaveformDestructiveEditKind::ReverseSelection)
            || target == WaveformDestructiveEditTarget::PlaySelection
        {
            return Err(format!("Load a sample before {}", kind.gerund_label()));
        }

        if matches!(kind, WaveformDestructiveEditKind::ReverseSelection)
            && target == WaveformDestructiveEditTarget::ActiveSelection
        {
            return self.selected_file_reverse_edit_target();
        }

        Err(target.missing_selection_message(kind))
    }

    fn selected_file_reverse_edit_target(&self) -> Result<(PathBuf, SelectionRange), String> {
        let absolute_path = self
            .library
            .folder_browser
            .selected_file_id()
            .map(PathBuf::from)
            .ok_or_else(|| {
                String::from("Mark an edit or play range or select a sample before reversing")
            })?;
        Ok((absolute_path, SelectionRange::new(0.0, 1.0)))
    }

    fn destructive_edit_selection_for_kind(
        &self,
        kind: WaveformDestructiveEditKind,
        target: WaveformDestructiveEditTarget,
    ) -> Result<Option<SelectionRange>, String> {
        if target == WaveformDestructiveEditTarget::PlaySelection {
            return Ok(self
                .waveform
                .current
                .play_selection()
                .filter(|selection| selection.width() > 0.0));
        }
        if matches!(kind, WaveformDestructiveEditKind::SlideSampleAudio { .. }) {
            return Ok(Some(SelectionRange::new(0.0, 1.0)));
        }
        if matches!(kind, WaveformDestructiveEditKind::ApplyEditSelectionEffects) {
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
            return Ok(Some(selection));
        }
        Ok(self.waveform.current.destructive_edit_selection())
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
            Self::ReverseSelection => "Reverse",
            Self::MuteSelection => "Mute",
            Self::ExtractAndTrimSelection => "Extract and trim",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
            Self::SlideSampleAudio { .. } => "Slide",
        }
    }

    fn gerund_label(self) -> &'static str {
        match self {
            Self::CropSelection => "cropping",
            Self::TrimSelection => "trimming",
            Self::ReverseSelection => "reversing",
            Self::MuteSelection => "muting",
            Self::ExtractAndTrimSelection => "extracting and trimming",
            Self::ApplyEditSelectionEffects => "applying edit mark edits",
            Self::SlideSampleAudio { .. } => "sliding",
        }
    }

    fn past_tense_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Cropped",
            Self::TrimSelection => "Trimmed",
            Self::ReverseSelection => "Reversed",
            Self::MuteSelection => "Muted",
            Self::ExtractAndTrimSelection => "Extracted and trimmed",
            Self::ApplyEditSelectionEffects => "Applied edit mark edits to",
            Self::SlideSampleAudio { .. } => "Slid",
        }
    }

    fn transaction_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Crop waveform selection",
            Self::TrimSelection => "Trim waveform selection",
            Self::ReverseSelection => "Reverse waveform selection",
            Self::MuteSelection => "Mute waveform selection",
            Self::ExtractAndTrimSelection => "Extract and trim waveform selection",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
            Self::SlideSampleAudio { .. } => "Slide sample audio",
        }
    }

    fn undo_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Restore cropped audio",
            Self::TrimSelection => "Restore trimmed audio",
            Self::ReverseSelection => "Restore reversed audio",
            Self::MuteSelection => "Restore muted audio",
            Self::ExtractAndTrimSelection => "Restore extracted and trimmed audio",
            Self::ApplyEditSelectionEffects => "Restore edit mark edits",
            Self::SlideSampleAudio { .. } => "Restore slid audio",
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
        WaveformDestructiveEditKind::ReverseSelection => {
            "This will reverse the selected region in place, or the selected file when no region is marked."
        }
        WaveformDestructiveEditKind::MuteSelection => {
            "This will silence the selected region in place without changing its duration."
        }
        WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            "This will extract the selected region into a new sibling file, then remove that region and close the gap in the source file."
        }
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            "This will overwrite the edit selection with the currently previewed fade and gain edits."
        }
        WaveformDestructiveEditKind::SlideSampleAudio { frame_offset } => {
            let direction = if frame_offset > 0 { "right" } else { "left" };
            return crate::native_app::app::WaveformDestructiveEditPrompt {
                edit,
                title: destructive_edit_title(edit),
                message: format!(
                    "This will circularly slide the source file audio {direction} by {} frame{} without changing its duration. Wavecrate will rewrite the file using the current write format: {}.",
                    frame_offset.unsigned_abs(),
                    if frame_offset.unsigned_abs() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    write_format.summary_label()
                ),
            };
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

fn next_copy_edit_path(
    source_path: &Path,
    target_folder: &Path,
    kind: WaveformDestructiveEditKind,
) -> Result<PathBuf, String> {
    let base_suffix = match kind {
        WaveformDestructiveEditKind::TrimSelection => "_trim",
        WaveformDestructiveEditKind::ReverseSelection => "_reverse",
        WaveformDestructiveEditKind::MuteSelection => "_mute",
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => "_edit",
        WaveformDestructiveEditKind::SlideSampleAudio { .. } => "_slide",
        WaveformDestructiveEditKind::CropSelection
        | WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            return Err(String::from("Unsupported protected copy edit"));
        }
    };
    wavecrate::sample_sources::harvest_file_ops::next_available_wav_copy_path(
        source_path,
        target_folder,
        base_suffix,
        "Could not find an available edit copy file name",
    )
}

fn destructive_edit_title(edit: WaveformDestructiveEditKind) -> String {
    match edit {
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            String::from("Apply edit mark edits")
        }
        _ => format!("{} selection", edit.action_label()),
    }
}
