use super::super::*;

pub(super) fn action_slug(action: &UiAction) -> Option<&'static str> {
    Some(match action {
        UiAction::NormalizeFocusedContentItem => "normalize_focused_content_item",
        UiAction::NormalizeWaveformSelectionOrLoadedContent => {
            "normalize_waveform_selection_or_loaded_content"
        }
        UiAction::CropWaveformSelection => "crop_waveform_selection",
        UiAction::CropWaveformSelectionToNewContentItem => {
            "crop_waveform_selection_to_new_content_item"
        }
        UiAction::TrimWaveformSelection => "trim_waveform_selection",
        UiAction::ReverseWaveformSelection => "reverse_waveform_selection",
        UiAction::FadeWaveformSelectionLeftToRight => "fade_waveform_selection_left_to_right",
        UiAction::FadeWaveformSelectionRightToLeft => "fade_waveform_selection_right_to_left",
        UiAction::MuteWaveformSelection => "mute_waveform_selection",
        UiAction::DeleteSelectedSliceMarkers => "delete_selected_slice_markers",
        UiAction::ToggleWaveformSliceSelection { .. } => "toggle_waveform_slice_selection",
        UiAction::AuditionWaveformDuplicateSlice { .. } => "audition_waveform_duplicate_slice",
        UiAction::ToggleWaveformDuplicateSliceExemption { .. } => {
            "toggle_waveform_duplicate_slice_exemption"
        }
        UiAction::MoveWaveformSliceFocus { .. } => "move_waveform_slice_focus",
        UiAction::ToggleFocusedWaveformSliceExportMark => {
            "toggle_focused_waveform_slice_export_mark"
        }
        UiAction::AlignWaveformStartToMarker => "align_waveform_start_to_marker",
        UiAction::DeleteLoadedWaveformContent => "delete_loaded_waveform_content",
        UiAction::SlideWaveformSelection { .. } => "slide_waveform_selection",
        UiAction::SetInputMonitoringEnabled { .. } => "set_input_monitoring_enabled",
        UiAction::SetAdvanceAfterRatingEnabled { .. } => "set_advance_after_rating_enabled",
        UiAction::SetDestructiveYoloMode { .. } => "set_destructive_yolo_mode",
        UiAction::SetInvertWaveformScroll { .. } => "set_invert_waveform_scroll",
        UiAction::ToggleLoopPlayback => "toggle_loop_playback",
        UiAction::ToggleLoopLock => "toggle_loop_lock",
        UiAction::ToggleTransientMarkers => "toggle_transient_markers",
        UiAction::ToggleBpmSnap => "toggle_bpm_snap",
        UiAction::SetWaveformChannelView { .. } => "set_waveform_channel_view",
        UiAction::SetNormalizedAuditionEnabled { .. } => "set_normalized_audition_enabled",
        UiAction::SetBpmSnapEnabled { .. } => "set_bpm_snap_enabled",
        UiAction::SetRelativeBpmGridEnabled { .. } => "set_relative_bpm_grid_enabled",
        UiAction::AdjustWaveformBpm { .. } => "adjust_waveform_bpm",
        UiAction::SetWaveformBpmValue { .. } => "set_waveform_bpm_value",
        UiAction::SetTransientSnapEnabled { .. } => "set_transient_snap_enabled",
        UiAction::SetTransientMarkersEnabled { .. } => "set_transient_markers_enabled",
        UiAction::SetSliceModeEnabled { .. } => "set_slice_mode_enabled",
        UiAction::SetVolume { .. } => "set_volume",
        UiAction::CommitVolumeSetting => "commit_volume_setting",
        UiAction::SeekWaveformPrecise { .. } => "seek_waveform_precise",
        UiAction::SetWaveformCursorPrecise { .. } => "set_waveform_cursor_precise",
        UiAction::SeekWaveform { .. } => "seek_waveform",
        UiAction::SetWaveformCursor { .. } => "set_waveform_cursor",
        UiAction::BeginWaveformCircularSlide { .. } => "begin_waveform_circular_slide",
        UiAction::UpdateWaveformCircularSlide { .. } => "update_waveform_circular_slide",
        UiAction::FinishWaveformCircularSlide => "finish_waveform_circular_slide",
        UiAction::BeginWaveformSelectionAt { .. } => "begin_waveform_selection_at",
        UiAction::BeginWaveformSelectionAtPrecise { .. } => "begin_waveform_selection_at_precise",
        UiAction::SetWaveformSelectionRange { .. } => "set_waveform_selection_range",
        UiAction::SetWaveformSelectionRangePrecise { .. } => "set_waveform_selection_range_precise",
        UiAction::SetWaveformSelectionRangeSmartScale { .. } => {
            "set_waveform_selection_range_smart_scale"
        }
        UiAction::SetWaveformSelectionRangeSmartScalePrecise { .. } => {
            "set_waveform_selection_range_smart_scale_precise"
        }
        UiAction::SetWaveformEditSelectionRange { .. } => "set_waveform_edit_selection_range",
        UiAction::SetWaveformEditSelectionRangePrecise { .. } => {
            "set_waveform_edit_selection_range_precise"
        }
        UiAction::SetWaveformEditFadeInEnd { .. } => "set_waveform_edit_fade_in_end",
        UiAction::SetWaveformEditFadeInMuteStart { .. } => "set_waveform_edit_fade_in_mute_start",
        UiAction::SetWaveformEditFadeInCurve { .. } => "set_waveform_edit_fade_in_curve",
        UiAction::SetWaveformEditFadeOutStart { .. } => "set_waveform_edit_fade_out_start",
        UiAction::SetWaveformEditFadeOutMuteEnd { .. } => "set_waveform_edit_fade_out_mute_end",
        UiAction::SetWaveformEditFadeOutCurve { .. } => "set_waveform_edit_fade_out_curve",
        UiAction::FinishWaveformEditFadeDrag => "finish_waveform_edit_fade_drag",
        UiAction::StartWaveformSelectionDrag { .. } => "start_waveform_selection_drag",
        UiAction::UpdateWaveformSelectionDrag { .. } => "update_waveform_selection_drag",
        UiAction::FinishWaveformSelectionDrag => "finish_waveform_selection_drag",
        UiAction::FinishWaveformSelectionRangeDrag => "finish_waveform_selection_range_drag",
        UiAction::FinishWaveformSelectionSmartScaleDrag => {
            "finish_waveform_selection_smart_scale_drag"
        }
        UiAction::BeginWaveformSelectionShift { .. } => "begin_waveform_selection_shift",
        UiAction::BeginWaveformSelectionShiftPrecise { .. } => {
            "begin_waveform_selection_shift_precise"
        }
        UiAction::BeginWaveformEditSelectionShift { .. } => "begin_waveform_edit_selection_shift",
        UiAction::BeginWaveformEditSelectionShiftPrecise { .. } => {
            "begin_waveform_edit_selection_shift_precise"
        }
        UiAction::FinishWaveformEditSelectionDrag => "finish_waveform_edit_selection_drag",
        UiAction::ClearWaveformSelection => "clear_waveform_selection",
        UiAction::ClearWaveformEditSelection => "clear_waveform_edit_selection",
        UiAction::ClearWaveformSelections => "clear_waveform_selections",
        UiAction::SetWaveformViewCenter { .. } => "set_waveform_view_center",
        UiAction::ZoomWaveform { .. } => "zoom_waveform",
        UiAction::ZoomWaveformToSelection => "zoom_waveform_to_selection",
        UiAction::ZoomWaveformFull => "zoom_waveform_full",
        _ => return None,
    })
}
