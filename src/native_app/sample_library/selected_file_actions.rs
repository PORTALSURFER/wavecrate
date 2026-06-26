use crate::native_app::app::{ExtractedFilePlaybackType, GuiMessage, NativeAppState};
use crate::native_app::app::{emit_gui_action, sample_path_label};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT, SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
};
use crate::native_app::waveform::{
    WaveformExtractionCompletion, WaveformExtractionRequest, WaveformSelectionKind,
    execute_waveform_extraction,
};
use radiant::gui::types::Point;
use std::time::Instant;
use wavecrate::sample_sources::HarvestDerivationOperation;

#[derive(Clone, Copy)]
enum PlaymarkedExtractionTarget {
    Default,
    HarvestDestination,
}

impl PlaymarkedExtractionTarget {
    fn action_name(self) -> &'static str {
        match self {
            Self::Default => "waveform.extract_playmarked_range",
            Self::HarvestDestination => "waveform.extract_playmarked_range.harvest_destination",
        }
    }

    fn status_text(self) -> &'static str {
        match self {
            Self::Default => "Extracting play range",
            Self::HarvestDestination => "Extracting play range to harvest destination",
        }
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn focus_loaded_file(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.waveform.current.has_loaded_sample() {
            self.ui.status.sample = String::from("Load a sample to focus it");
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }
        let path = self.waveform.current.path();
        if self
            .library
            .folder_browser
            .focus_file_across_sources_matching_tags(&path, &self.metadata.tags_by_file)
        {
            if let Some(index) = self
                .library
                .folder_browser
                .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
            {
                context.scroll_into_view_snapped(
                    SAMPLE_BROWSER_LIST_ID,
                    index as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_ROW_HEIGHT,
                );
            }
            self.ui.status.sample = format!("Focused {}", sample_path_label(&path));
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "success",
                started_at,
                None,
            );
        } else {
            let error = format!(
                "Loaded sample is not visible in sources: {}",
                path.display()
            );
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "not_found",
                started_at,
                Some(&error),
            );
        }
    }

    pub(in crate::native_app) fn delete_selected_item(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        if self.library.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files(context);
        } else {
            self.delete_selected_folder(context);
        }
    }

    fn delete_selected_folder(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let target = match self.library.folder_browser.selected_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.move_selected_folder_to_trash(target.path, started_at, context);
    }

    fn delete_selected_files(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let target = match self.library.folder_browser.selected_file_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.move_selected_files_to_trash(target.paths, started_at, context);
    }

    pub(in crate::native_app) fn extract_playmarked_range(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.extract_playmarked_range_to_target(context, PlaymarkedExtractionTarget::Default);
    }

    pub(in crate::native_app) fn extract_playmarked_range_to_harvest_destination(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.extract_playmarked_range_to_target(
            context,
            PlaymarkedExtractionTarget::HarvestDestination,
        );
    }

    fn extract_playmarked_range_to_target(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
        target: PlaymarkedExtractionTarget,
    ) {
        let started_at = Instant::now();
        let action = target.action_name();
        match self
            .waveform
            .current
            .play_selection_extraction_request(None)
        {
            Ok(request) => {
                let selection = request.selection();
                let request = match self.route_playmarked_extraction_request(request, target) {
                    Ok(request) => request,
                    Err(error) => {
                        self.flash_denied_waveform_selection_for_error(
                            &error,
                            Some(selection),
                            WaveformSelectionKind::Play,
                        );
                        self.ui.status.sample = error.clone();
                        emit_gui_action(
                            action,
                            Some("waveform"),
                            None,
                            "blocked",
                            started_at,
                            Some(&error),
                        );
                        return;
                    }
                };
                let playback_type =
                    ExtractedFilePlaybackType::from_loop_active(self.audio.loop_playback);
                if let Err(error) = self.validate_waveform_extraction_target(&request) {
                    self.flash_denied_waveform_selection_for_error(
                        &error,
                        Some(request.selection()),
                        WaveformSelectionKind::Play,
                    );
                    self.ui.status.sample = error.clone();
                    emit_gui_action(
                        action,
                        Some("waveform"),
                        None,
                        "blocked",
                        started_at,
                        Some(&error),
                    );
                    return;
                }
                self.ui.status.sample = String::from(target.status_text());
                context.business().background("gui-waveform-extract").run(
                    move |_| execute_waveform_extraction(request),
                    move |completion| GuiMessage::PlaySelectionExtractionFinished {
                        completion,
                        drag_position: None,
                        playback_type,
                        harvest_operation: HarvestDerivationOperation::Extract,
                        started_at,
                    },
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn route_playmarked_extraction_request(
        &self,
        request: WaveformExtractionRequest,
        target: PlaymarkedExtractionTarget,
    ) -> Result<WaveformExtractionRequest, String> {
        match target {
            PlaymarkedExtractionTarget::Default => self.route_harvest_extraction_request(request),
            PlaymarkedExtractionTarget::HarvestDestination => {
                self.route_harvest_destination_extraction_request(request)
            }
        }
    }

    pub(in crate::native_app) fn route_harvest_extraction_request(
        &self,
        request: WaveformExtractionRequest,
    ) -> Result<WaveformExtractionRequest, String> {
        if request.has_explicit_target_folder() {
            return Ok(request);
        }
        let Some(target_folder) =
            self.optional_harvest_destination_for_protected_origin(request.source_path())
        else {
            return Ok(request);
        };
        wavecrate::sample_sources::harvest_file_ops::ensure_dir(
            &target_folder,
            "Could not create harvest destination",
        )?;
        Ok(request.with_target_folder(target_folder))
    }

    pub(in crate::native_app) fn route_harvest_destination_extraction_request(
        &self,
        request: WaveformExtractionRequest,
    ) -> Result<WaveformExtractionRequest, String> {
        let target_folder = self.harvest_destination_for_origin(request.source_path())?;
        wavecrate::sample_sources::harvest_file_ops::ensure_dir(
            &target_folder,
            "Could not create harvest destination",
        )?;
        Ok(request.with_target_folder(target_folder))
    }

    pub(in crate::native_app) fn validate_waveform_extraction_target(
        &self,
        request: &WaveformExtractionRequest,
    ) -> Result<(), String> {
        let target_folder = request.target_folder()?;
        if let Some(error) = self
            .library
            .folder_browser
            .folder_target_lock_error(target_folder, "Extraction")
        {
            return Err(error);
        }
        Ok(())
    }

    pub(in crate::native_app) fn finish_play_selection_extraction(
        &mut self,
        completion: WaveformExtractionCompletion,
        drag_position: Option<Point>,
        playback_type: ExtractedFilePlaybackType,
        harvest_operation: HarvestDerivationOperation,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        match completion.result {
            Ok(path) => {
                self.waveform
                    .current
                    .mark_extracted_play_selection(&completion.source_path, completion.selection);
                self.waveform.current.flash_play_selection();
                let protected_origin = self
                    .library
                    .folder_browser
                    .path_is_in_protected_source(&completion.source_path);
                let cross_source_derivative =
                    self.extraction_derivative_crosses_sources(&completion.source_path, &path);
                let focus_derivative = protected_origin || cross_source_derivative;
                if focus_derivative {
                    self.library
                        .folder_browser
                        .refresh_file_path_across_sources(&path);
                } else {
                    self.library.folder_browser.refresh_file_path(&path);
                }
                let metadata_error = self
                    .assign_extracted_file_metadata(&path, playback_type, context)
                    .err();
                self.record_harvest_selection_derivation_with_source_duration(
                    &completion.source_path,
                    completion.selection,
                    &path,
                    self.waveform.current.duration_seconds() as f64,
                    harvest_operation,
                );
                if let Some(position) = drag_position {
                    self.library
                        .folder_browser
                        .begin_extracted_file_drag(path.clone(), position);
                    self.arm_browser_drag(context);
                    let label = sample_path_label(&path);
                    self.ui.status.sample = match metadata_error {
                        Some(error) => {
                            format!("Dragging {label}; extracted metadata incomplete: {error}")
                        }
                        None => format!("Dragging {label}"),
                    };
                    emit_gui_action(
                        "waveform.selection_drag.start",
                        Some("waveform"),
                        None,
                        "success",
                        started_at,
                        None,
                    );
                } else {
                    if focus_derivative {
                        self.library
                            .folder_browser
                            .focus_file_across_sources_matching_tags(
                                &path,
                                &self.metadata.tags_by_file,
                            );
                        self.load_navigation_sample_validated(
                            path.to_string_lossy().to_string(),
                            context,
                            started_at,
                        );
                    }
                    let label = sample_path_label(&path);
                    self.ui.status.sample = match metadata_error {
                        Some(error) => {
                            format!("Extracted {label}; extracted metadata incomplete: {error}")
                        }
                        None => format!("Extracted {label}"),
                    };
                    emit_gui_action(
                        "waveform.extract_playmarked_range",
                        Some("waveform"),
                        Some(&label),
                        "success",
                        started_at,
                        None,
                    );
                }
            }
            Err(error) => {
                let action = if drag_position.is_some() {
                    "waveform.selection_drag.start"
                } else {
                    "waveform.extract_playmarked_range"
                };
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn extraction_derivative_crosses_sources(
        &self,
        source_path: &std::path::Path,
        child_path: &std::path::Path,
    ) -> bool {
        let source_id = self
            .library
            .folder_browser
            .sample_source_for_file_path(source_path)
            .map(|(source, _)| source.id);
        let child_id = self
            .library
            .folder_browser
            .sample_source_for_file_path(child_path)
            .map(|(source, _)| source.id);
        source_id.is_some() && child_id.is_some() && source_id != child_id
    }
}
