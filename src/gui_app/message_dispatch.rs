use radiant::prelude as ui;
use std::time::Instant;

use super::{
    AudioSettingsDropdown, GuiAppState, GuiMessage, WaveformActiveDragKind, WaveformInteraction,
    WaveformSelectionKind, emit_gui_action,
};

impl GuiAppState {
    fn toggle_audio_backend_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::Backend);
    }

    fn toggle_audio_output_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::Output);
    }

    fn toggle_audio_sample_rate_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::SampleRate);
    }

    fn apply_waveform_message(
        &mut self,
        message: WaveformInteraction,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let action = waveform_interaction_action(&message);
        let active_drag = self.waveform.active_drag_kind();
        if let WaveformInteraction::DragPlaySelectionExport(drag) = message
            && !self.drag_waveform_play_selection(drag, context)
        {
            return;
        }
        self.waveform.apply_interaction(message);
        self.sync_edit_fade_audio_state();
        if waveform_interaction_finishes_play_selection_edit(&message, active_drag) {
            self.retarget_loop_playback_to_play_selection();
        }
        if let Some(action) = action {
            emit_gui_action(action, Some("waveform"), None, "applied", started_at, None);
        }
        if let Some(start_ratio) = self.waveform.take_pending_playback_start() {
            self.play_waveform_from_ratio(start_ratio);
        }
    }

    pub(in crate::gui_app) fn apply_message(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::AddSourceDialogFinished(result) => {
                self.finish_add_source_dialog(result, context);
            }
            GuiMessage::FolderBrowser(message) => {
                self.apply_folder_browser_message(message, context);
            }
            GuiMessage::FolderScanProgress(progress) => {
                self.apply_folder_scan_progress(progress);
            }
            GuiMessage::FolderScanDiscoveryBatch(batch) => {
                self.apply_folder_scan_discovery_batch(batch);
            }
            GuiMessage::FolderScanFinished(result) => self.finish_folder_scan(result),
            GuiMessage::NormalizationProgress(progress) => {
                self.apply_normalization_progress(progress);
            }
            GuiMessage::NormalizationFinished(result) => self.finish_normalization(result),
            GuiMessage::SelectSampleWithModifiers { path, modifiers } => {
                self.context_menu = None;
                self.select_sample_with_modifiers(path, modifiers, context);
            }
            GuiMessage::OpenSampleContextMenu { path, position } => {
                self.open_sample_context_menu(path, position);
            }
            GuiMessage::DragSampleFile { path, drag } => {
                self.context_menu = None;
                self.drag_sample_file(path, drag, context);
            }
            GuiMessage::ExternalDragCompleted(result) => {
                self.external_drag_completed(result, context)
            }
            GuiMessage::DeferredSampleLoad {
                ticket,
                path,
                autoplay,
            } => {
                self.start_deferred_sample_load(ticket, path, autoplay, context);
            }
            GuiMessage::SampleLoadProgress(ticket, progress) => {
                if self.sample_load_task.is_active(ticket) {
                    self.waveform_loading_target_progress = progress.clamp(0.0, 0.995);
                }
            }
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result),
            GuiMessage::AudioPlayerOpenFinished(ticket) => self.finish_audio_player_open(ticket),
            GuiMessage::PlaySelectedSample => self.play_selected_sample(context),
            GuiMessage::StopPlayback => self.stop_playback(),
            GuiMessage::ToggleLoopPlayback => self.toggle_loop_playback(),
            GuiMessage::SetVolume(volume) => self.set_volume(volume),
            GuiMessage::ToggleAudioSettings => self.toggle_audio_settings(),
            GuiMessage::CloseAudioSettings => {
                self.close_audio_settings_window();
            }
            GuiMessage::ToggleAudioBackendDropdown => {
                self.toggle_audio_backend_dropdown();
            }
            GuiMessage::ToggleAudioOutputDropdown => {
                self.toggle_audio_output_dropdown();
            }
            GuiMessage::ToggleAudioSampleRateDropdown => {
                self.toggle_audio_sample_rate_dropdown();
            }
            GuiMessage::CloseAudioSettingsDropdowns => {
                self.close_audio_settings_dropdowns();
            }
            GuiMessage::SetAudioOutputHost(host) => self.set_audio_output_host(host),
            GuiMessage::SetAudioOutputDevice(device) => self.set_audio_output_device(device),
            GuiMessage::SetAudioOutputSampleRate(sample_rate) => {
                self.set_audio_output_sample_rate(sample_rate);
            }
            GuiMessage::MetadataTagInput(message) => {
                self.apply_metadata_tag_input(message, context)
            }
            GuiMessage::MoveMetadataTagCompletion(delta) => {
                self.move_metadata_tag_completion_selection(delta);
            }
            GuiMessage::ToggleMetadataTagLibrary => self.toggle_metadata_tag_library(),
            GuiMessage::ToggleMetadataTagCategory(category_id) => {
                self.toggle_metadata_tag_category(category_id);
            }
            GuiMessage::SelectMetadataTag(tag) => {
                self.select_metadata_tag(tag);
            }
            GuiMessage::ToggleMetadataTag(tag) => {
                self.toggle_metadata_tag(tag, context);
            }
            GuiMessage::DragMetadataTag { tag, drag } => {
                self.drag_metadata_tag(tag, drag, context);
            }
            GuiMessage::HoverMetadataTagDropCategory { category_id } => {
                self.hover_metadata_tag_drop_category(category_id);
            }
            GuiMessage::DropMetadataTagOnCategory { category_id } => {
                self.drop_metadata_tag_on_category(category_id, context);
            }
            GuiMessage::OpenMetadataTagContextMenu { tag, position } => {
                self.open_metadata_tag_context_menu(tag, position);
            }
            GuiMessage::DeleteContextMetadataTag => {
                self.delete_context_metadata_tag(context);
            }
            GuiMessage::DeleteSelectedMetadataTag => {
                self.remove_selected_metadata_tag(context);
            }
            GuiMessage::MetadataTagsPersisted(result) => self.finish_metadata_tag_persist(result),
            GuiMessage::ToggleSampleNameViewMode => {
                self.sample_name_view_mode = self.sample_name_view_mode.toggled();
            }
            GuiMessage::ClearRebuildableCaches => self.clear_rebuildable_caches(),
            GuiMessage::FocusLoadedFile => self.focus_loaded_file(context),
            GuiMessage::AdjustSelectedRating(delta) => self.adjust_selected_rating(delta, context),
            GuiMessage::AssignSelectedCollection(collection) => {
                self.assign_selected_collection(collection, context)
            }
            GuiMessage::NormalizeSelectedSamples => self.normalize_selected_samples(context),
            GuiMessage::CopySelectedFiles => self.copy_selected_files(),
            GuiMessage::CopyContextPath => self.copy_context_path(context),
            GuiMessage::ContextPathCopyFinished { kind, path, result } => {
                self.finish_context_path_copy(kind, path, result);
            }
            GuiMessage::OpenContextTarget => self.open_context_target(context),
            GuiMessage::ContextTargetOpenFinished { kind, path, result } => {
                self.finish_context_target_open(kind, path, result);
            }
            GuiMessage::RemoveContextSource => self.remove_context_source(),
            GuiMessage::CloseContextMenu => {
                self.context_menu = None;
            }
            GuiMessage::ToggleJobDetails => {
                self.job_details_open = self.folder_progress.is_some() && !self.job_details_open;
            }
            GuiMessage::CloseJobDetails => {
                self.job_details_open = false;
            }
            GuiMessage::Noop => {}
            GuiMessage::FocusRenameInput(input_id) => {
                self.focus_rename_input(input_id, context);
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(),
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(),
            GuiMessage::NavigateBrowser { delta, extend } => {
                self.navigate_browser(delta, extend, context);
            }
            GuiMessage::SelectAllSamples => {
                self.select_all_samples();
            }
            GuiMessage::CollapseSelectedFolder => {
                self.collapse_selected_folder();
            }
            GuiMessage::ExpandSelectedFolder => {
                self.expand_selected_folder();
            }
            GuiMessage::DropWaveformSelectionOnSampleList => {
                self.drop_waveform_play_selection_on_sample_list(context);
            }
            GuiMessage::Waveform(message) => {
                self.apply_waveform_message(message, context);
            }
            GuiMessage::NativeFileDrop(drop) => self.apply_native_file_drop(drop, context),
            GuiMessage::Frame => {
                self.maybe_open_audio_player(context);
                self.maybe_startup_source_scan(context);
                self.maybe_auto_load_startup_sample(context);
                self.advance_frame();
            }
        }
    }

    fn toggle_metadata_tag_library(&mut self) {
        let started_at = Instant::now();
        self.metadata_tag_library_open = !self.metadata_tag_library_open;
        let outcome = if self.metadata_tag_library_open {
            "opened"
        } else {
            "closed"
        };
        emit_gui_action(
            "metadata_tags.toggle_library",
            Some("folder_browser"),
            None,
            outcome,
            started_at,
            None,
        );
    }

    fn toggle_metadata_tag_category(&mut self, category_id: String) {
        let started_at = Instant::now();
        let source = category_id.clone();
        if !self.collapsed_metadata_tag_categories.remove(&category_id) {
            self.collapsed_metadata_tag_categories.insert(category_id);
        }
        emit_gui_action(
            "metadata_tags.toggle_category",
            Some("tag_editor"),
            Some(source.as_str()),
            "applied",
            started_at,
            None,
        );
    }
}

fn waveform_interaction_finishes_play_selection_edit(
    interaction: &WaveformInteraction,
    active_drag: Option<WaveformActiveDragKind>,
) -> bool {
    if !matches!(interaction, WaveformInteraction::FinishSelection { .. }) {
        return false;
    }
    matches!(
        active_drag,
        Some(WaveformActiveDragKind::Selection(
            WaveformSelectionKind::Play
        )) | Some(WaveformActiveDragKind::SelectionResize(
            WaveformSelectionKind::Play,
            _
        )) | Some(WaveformActiveDragKind::SelectionMove(
            WaveformSelectionKind::Play
        ))
    )
}

fn waveform_interaction_action(interaction: &WaveformInteraction) -> Option<&'static str> {
    match interaction {
        WaveformInteraction::Wheel { .. } => Some("waveform.zoom_wheel"),
        WaveformInteraction::ScrollTo { .. } => Some("waveform.scroll"),
        WaveformInteraction::BeginSelection { .. } => Some("waveform.selection.begin"),
        WaveformInteraction::BeginEditFade { .. } => Some("waveform.edit_fade.begin"),
        WaveformInteraction::ClearEditFadeSilence { .. } => {
            Some("waveform.edit_fade.clear_silence")
        }
        WaveformInteraction::BeginSelectionResize { .. } => Some("waveform.selection.resize_begin"),
        WaveformInteraction::BeginSelectionMove { .. } => Some("waveform.selection.move_begin"),
        WaveformInteraction::BeginPan { .. } => Some("waveform.pan_begin"),
        WaveformInteraction::DragPlaySelectionExport(drag) if drag.is_started() => {
            Some("waveform.selection_export_drag.begin")
        }
        WaveformInteraction::DragPlaySelectionExport(drag) if drag.is_ended() => {
            Some("waveform.selection_export_drag.end")
        }
        WaveformInteraction::FinishSelection { .. } => Some("waveform.selection.finish"),
        WaveformInteraction::UpdateSelection { .. }
        | WaveformInteraction::DragPlaySelectionExport(_)
        | WaveformInteraction::Frame => None,
    }
}
