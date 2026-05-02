//! Shared helper utilities for semantic automation snapshot builders.

use super::*;
use crate::compat_app_contract::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    NormalizedRangeModel, UpdateStatusModel,
};
use std::collections::BTreeMap;

/// Build one leaf automation node with empty metadata and no children.
pub(super) fn simple_node(
    id: impl Into<String>,
    role: AutomationRole,
    label: Option<String>,
    rect: Rect,
    value: Option<String>,
    enabled: bool,
    selected: bool,
    available_actions: Vec<String>,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: node_id(id),
        role,
        label,
        bounds: bounds(rect),
        value,
        enabled,
        selected,
        available_actions,
        metadata: BTreeMap::new(),
        children: Vec::new(),
    }
}

/// Build one stable automation node id.
pub(super) fn node_id(id: impl Into<String>) -> AutomationNodeId {
    AutomationNodeId::new(id)
}

/// Convert one UI rectangle into deterministic automation bounds.
pub(super) fn bounds(rect: Rect) -> AutomationBounds {
    AutomationBounds {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

/// Build metadata while omitting empty values.
pub(super) fn metadata(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| (String::from(*key), String::from(*value)))
        .collect()
}

/// Return a stable textual boolean for semantic metadata.
pub(super) fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Return a stable update-status label for semantic metadata.
pub(super) fn update_status_text(status: UpdateStatusModel) -> &'static str {
    match status {
        UpdateStatusModel::Idle => "idle",
        UpdateStatusModel::Checking => "checking",
        UpdateStatusModel::Available => "available",
        UpdateStatusModel::Error => "error",
    }
}

/// Return a stable selection range string in microseconds.
pub(super) fn selection_micros_text(range: NormalizedRangeModel) -> String {
    format!("{}-{}", range.start_micros, range.end_micros)
}

/// Convert a UI label into a stable automation slug.
pub(super) fn slug(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Convert one concrete action into its stable automation action id.
pub(super) fn action_slug(action: &UiAction) -> String {
    match action {
        UiAction::SelectColumn { .. } => "select_column",
        UiAction::MoveColumn { .. } => "move_column",
        UiAction::ToggleTransport => "toggle_transport",
        UiAction::PlayCompareAnchor => "play_compare_anchor",
        UiAction::PlayFromStart => "play_from_start",
        UiAction::PlayFromCurrentPlayhead => "play_from_current_playhead",
        UiAction::PlayFromWaveformCursor => "play_from_waveform_cursor",
        UiAction::PlayWaveformAtPrecise { .. } => "play_waveform_at_precise",
        UiAction::HandleEscape => "handle_escape",
        UiAction::FocusBrowserPanel => "focus_browser_panel",
        UiAction::FocusSourcesPanel => "focus_sources_panel",
        UiAction::FocusWaveformPanel => "focus_waveform_panel",
        UiAction::FocusFolderPanel { .. } => "focus_folder_panel",
        UiAction::FocusLoadedContentInList => "focus_loaded_content_in_list",
        UiAction::FocusBrowserSearch => "focus_browser_search",
        UiAction::BlurBrowserSearch => "blur_browser_search",
        UiAction::OpenAddSourceDialog => "open_add_source_dialog",
        UiAction::OpenOptionsMenu => "open_options_menu",
        UiAction::CloseOptionsPanel => "close_options_panel",
        UiAction::EditDefaultIdentifier => "edit_default_identifier",
        UiAction::ShowOptionsOverview => "show_options_overview",
        UiAction::PickTrashFolder => "pick_trash_folder",
        UiAction::OpenTrashFolder => "open_trash_folder",
        UiAction::OpenPrimaryGroupPicker => "open_primary_group_picker",
        UiAction::OpenPrimaryItemPicker => "open_primary_item_picker",
        UiAction::OpenPrimaryNumberPicker => "open_primary_number_picker",
        UiAction::OpenSecondaryGroupPicker => "open_secondary_group_picker",
        UiAction::OpenSecondaryItemPicker => "open_secondary_item_picker",
        UiAction::OpenSecondaryNumberPicker => "open_secondary_number_picker",
        UiAction::SetPrimaryGroup { .. } => "set_primary_group",
        UiAction::SetPrimaryItem { .. } => "set_primary_item",
        UiAction::SetPrimaryNumber { .. } => "set_primary_number",
        UiAction::SetSecondaryGroup { .. } => "set_secondary_group",
        UiAction::SetSecondaryItem { .. } => "set_secondary_item",
        UiAction::SetSecondaryNumber { .. } => "set_secondary_number",
        UiAction::FocusFolderSearch { .. } => "focus_folder_search",
        UiAction::SetFolderSearch { .. } => "set_folder_search",
        UiAction::ToggleShowAllFolders { .. } => "toggle_show_all_folders",
        UiAction::ToggleFolderFlattenedView { .. } => "toggle_folder_flattened_view",
        UiAction::SelectSourceRow { .. } => "select_source_row",
        UiAction::FocusSourceRow { .. } => "focus_source_row",
        UiAction::MoveSourceFocus { .. } => "move_source_focus",
        UiAction::ReloadFocusedSourceRow => "reload_focused_source_row",
        UiAction::HardSyncFocusedSourceRow => "hard_sync_focused_source_row",
        UiAction::OpenFocusedSourceFolder => "open_focused_source_folder",
        UiAction::RemoveFocusedSourceRow => "remove_focused_source_row",
        UiAction::ReloadSourceRow { .. } => "reload_source_row",
        UiAction::HardSyncSourceRow { .. } => "hard_sync_source_row",
        UiAction::OpenSourceFolderRow { .. } => "open_source_folder_row",
        UiAction::RemoveSourceRow { .. } => "remove_source_row",
        UiAction::FocusFolderRow { .. } => "focus_folder_row",
        UiAction::ActivateFolderRow { .. } => "activate_folder_row",
        UiAction::ToggleFolderRowExpanded { .. } => "toggle_folder_row_expanded",
        UiAction::ExpandFocusedFolder => "expand_focused_folder",
        UiAction::CollapseFocusedFolder => "collapse_focused_folder",
        UiAction::ToggleFocusedFolderSelection => "toggle_focused_folder_selection",
        UiAction::MoveFolderFocus { .. } => "move_folder_focus",
        UiAction::StartNewFolder => "start_new_folder",
        UiAction::StartNewFolderAtFolderRow { .. } => "start_new_folder_at_folder_row",
        UiAction::StartNewFolderAtRoot => "start_new_folder_at_root",
        UiAction::FocusFolderCreateInput => "focus_folder_create_input",
        UiAction::SetFolderCreateInput { .. } => "set_folder_create_input",
        UiAction::ConfirmFolderCreate => "confirm_folder_create",
        UiAction::CancelFolderCreate => "cancel_folder_create",
        UiAction::StartFolderRename => "start_folder_rename",
        UiAction::DeleteFocusedFolder => "delete_focused_folder",
        UiAction::RestoreRetainedFolderDeletes => "restore_retained_folder_deletes",
        UiAction::PurgeRetainedFolderDeletes => "purge_retained_folder_deletes",
        UiAction::ClearFolderDeleteRecoveryLog => "clear_folder_delete_recovery_log",
        UiAction::MoveBrowserFocus { .. } => "move_browser_focus",
        UiAction::SetBrowserViewStart { .. } => "set_browser_view_start",
        UiAction::FocusBrowserRow { .. } => "focus_browser_row",
        UiAction::AutoRenameBrowserSelection { .. } => "auto_rename_browser_selection",
        UiAction::SetCompareAnchorFromFocusedContent => "set_compare_anchor_from_focused_content",
        UiAction::CommitFocusedBrowserRow => "commit_focused_browser_row",
        UiAction::SaveWaveformSelectionToBrowser => "save_waveform_selection_to_browser",
        UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            "save_waveform_selection_to_browser_with_keep2"
        }
        UiAction::CommitWaveformEditFades => "commit_waveform_edit_fades",
        UiAction::DetectWaveformSilenceSlices => "detect_waveform_silence_slices",
        UiAction::DetectWaveformExactDuplicateSlices => "detect_waveform_exact_duplicate_slices",
        UiAction::CleanWaveformExactDuplicateSlices => "clean_waveform_exact_duplicate_slices",
        UiAction::ToggleBrowserRowSelection { .. } => "toggle_browser_row_selection",
        UiAction::StartContentItemDrag { .. } => "start_content_item_drag",
        UiAction::UpdateContentItemDrag { .. } => "update_content_item_drag",
        UiAction::FinishContentItemDrag => "finish_content_item_drag",
        UiAction::ExtendBrowserSelectionToRow { .. } => "extend_browser_selection_to_row",
        UiAction::AddRangeBrowserSelection { .. } => "add_range_browser_selection",
        UiAction::ExtendBrowserSelectionFromFocus { .. } => "extend_browser_selection_from_focus",
        UiAction::AddRangeBrowserSelectionFromFocus { .. } => {
            "add_range_browser_selection_from_focus"
        }
        UiAction::ToggleFocusedBrowserRowSelection => "toggle_focused_browser_row_selection",
        UiAction::SelectAllBrowserRows => "select_all_browser_rows",
        UiAction::SetBrowserSearch { .. } => "set_browser_search",
        UiAction::ToggleBrowserRatingFilter { .. } => "toggle_browser_rating_filter",
        UiAction::ToggleBrowserPlaybackAgeFilter { .. } => "toggle_browser_playback_age_filter",
        UiAction::ToggleContentMark => "toggle_content_mark",
        UiAction::ToggleBrowserMarkedFilter => "toggle_browser_marked_filter",
        UiAction::ToggleBrowserDerivedLabelFilter { .. } => "toggle_browser_derived_label_filter",
        UiAction::ToggleRandomNavigationMode => "toggle_random_navigation_mode",
        UiAction::ToggleBrowserPillEditor => "toggle_browser_pill_editor",
        UiAction::ToggleBrowserPillEditorPrimaryAction => {
            "toggle_browser_pill_editor_primary_action"
        }
        UiAction::ToggleBrowserDuplicateCleanupMode => "toggle_browser_duplicate_cleanup_mode",
        UiAction::FocusPreviousBrowserHistory => "focus_previous_browser_history",
        UiAction::FocusNextBrowserHistory => "focus_next_browser_history",
        UiAction::ToggleFindSimilarFocusedContent => "toggle_find_similar_focused_content",
        UiAction::ToggleBrowserDuplicateCleanupKeep { .. } => {
            "toggle_browser_duplicate_cleanup_keep"
        }
        UiAction::ConfirmBrowserDuplicateCleanup => "confirm_browser_duplicate_cleanup",
        UiAction::PlayRandomContentItem => "play_random_content_item",
        UiAction::PlayPreviousRandomContentItem => "play_previous_random_content_item",
        UiAction::AdjustSelectedBrowserRating { .. } => "adjust_selected_browser_rating",
        UiAction::SetBrowserTab { .. } => "set_browser_tab",
        UiAction::FocusBrowserPillEditorInput => "focus_browser_pill_editor_input",
        UiAction::SetBrowserPillEditorInput { .. } => "set_browser_pill_editor_input",
        UiAction::CommitBrowserPillEditorInput => "commit_browser_pill_editor_input",
        UiAction::SetBrowserSidebarLooped { .. } => "set_browser_sidebar_looped",
        UiAction::ToggleBrowserPillOption { .. } => "toggle_browser_pill_option",
        UiAction::FocusSpatialContentItem { .. } => "focus_spatial_content_item",
        UiAction::SetPromptInput { .. } => "set_prompt_input",
        UiAction::StartBrowserRename => "start_browser_rename",
        UiAction::ConfirmBrowserRename => "confirm_browser_rename",
        UiAction::CancelBrowserRename => "cancel_browser_rename",
        UiAction::SetBrowserTriageMark { .. } => "set_browser_triage_mark",
        UiAction::DeleteBrowserSelection => "delete_browser_selection",
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
        UiAction::ConfirmPrompt => "confirm_prompt",
        UiAction::CancelPrompt => "cancel_prompt",
        UiAction::CancelProgress => "cancel_progress",
        UiAction::CopySelectionToClipboard => "copy_selection_to_clipboard",
        UiAction::SetInputMonitoringEnabled { .. } => "set_input_monitoring_enabled",
        UiAction::SetAdvanceAfterRatingEnabled { .. } => "set_advance_after_rating_enabled",
        UiAction::SetDestructiveYoloMode { .. } => "set_destructive_yolo_mode",
        UiAction::SetInvertWaveformScroll { .. } => "set_invert_waveform_scroll",
        UiAction::ToggleLoopPlayback => "toggle_loop_playback",
        UiAction::ToggleLoopLock => "toggle_loop_lock",
        UiAction::ToggleTransientMarkers => "toggle_transient_markers",
        UiAction::ToggleBpmSnap => "toggle_bpm_snap",
        UiAction::ToggleHotkeyOverlay => "toggle_hotkey_overlay",
        UiAction::CopyStatusLog => "copy_status_log",
        UiAction::OpenFeedbackIssuePrompt => "open_feedback_issue_prompt",
        UiAction::MoveDiscardedItemsToFolder => "move_discarded_items_to_folder",
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
        UiAction::Undo => "undo",
        UiAction::Redo => "redo",
        UiAction::CheckForUpdates => "check_for_updates",
        UiAction::OpenUpdateLink => "open_update_link",
        UiAction::InstallUpdate => "install_update",
        UiAction::DismissUpdate => "dismiss_update",
    }
    .to_string()
}

/// Return a square rectangle centered on the supplied point.
pub(super) fn circle_rect(center: Point, diameter: f32) -> Rect {
    let radius = diameter * 0.5;
    Rect::from_min_max(
        Point::new(center.x - radius, center.y - radius),
        Point::new(center.x + radius, center.y + radius),
    )
}
