//! Waveform-oriented native action dispatch helpers.
//!
//! This keeps the main migration controller focused on orchestration while the
//! higher-churn waveform action surface stays isolated in a smaller module.

use super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::{DragSource, DragTarget, UiPoint};
use crate::app_core::state::{DestructiveSelectionEdit, StatusTone};

/// Try to dispatch waveform, zoom, and waveform-selection drag actions.
pub(super) fn apply_waveform_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SetWaveformChannelView { stereo } => {
            let view = if stereo {
                crate::waveform::WaveformChannelView::SplitStereo
            } else {
                crate::waveform::WaveformChannelView::Mono
            };
            controller.set_waveform_channel_view(view);
        }
        NativeUiAction::SetNormalizedAuditionEnabled { enabled } => {
            controller.set_normalized_audition_enabled(enabled)
        }
        NativeUiAction::AdjustWaveformBpm { delta } => adjust_waveform_bpm(controller, delta),
        NativeUiAction::SetWaveformBpmValue { value_tenths } => {
            controller.set_bpm_value(f32::from(value_tenths) / 10.0);
        }
        NativeUiAction::SetBpmSnapEnabled { enabled } => controller.set_bpm_snap_enabled(enabled),
        NativeUiAction::SetRelativeBpmGridEnabled { enabled } => {
            controller.set_relative_bpm_grid_enabled(enabled)
        }
        NativeUiAction::SetTransientSnapEnabled { enabled } => {
            controller.set_transient_snap_enabled(enabled)
        }
        NativeUiAction::SetTransientMarkersEnabled { enabled } => {
            controller.set_transient_markers_enabled(enabled)
        }
        NativeUiAction::SetSliceModeEnabled { enabled } => {
            if controller.loaded_waveform_slice_export_in_progress() {
                controller.set_status(
                    "Wait for the current slice export to finish",
                    StatusTone::Info,
                );
                return Ok(());
            }
            controller.set_slice_mode_enabled(enabled)
        }
        NativeUiAction::ToggleWaveformSliceSelection { index } => {
            controller.toggle_slice_selection(index);
            controller.focus_waveform_context();
        }
        NativeUiAction::AuditionWaveformDuplicateSlice { index } => {
            controller.audition_duplicate_cleanup_preview(index);
        }
        NativeUiAction::ToggleWaveformDuplicateSliceExemption { index } => {
            if let Err(err) = controller.toggle_duplicate_cleanup_preview_exemption(index) {
                controller.set_status(err, StatusTone::Info);
            }
            controller.focus_waveform_context();
        }
        NativeUiAction::MoveWaveformSliceFocus { delta } => {
            if !controller.move_slice_review_focus(delta) {
                controller.slide_selection_range(delta.into());
            }
        }
        NativeUiAction::ToggleFocusedWaveformSliceExportMark => {
            if let Err(err) = controller.toggle_focused_slice_export_mark() {
                controller.set_status(err, StatusTone::Info);
            }
            controller.focus_waveform_context();
        }
        NativeUiAction::SeekWaveformPrecise { position_nanos } => {
            controller.queue_waveform_seek_nanos(position_nanos)
        }
        NativeUiAction::SeekWaveform { position_milli } => {
            controller.queue_waveform_seek_milli(position_milli)
        }
        NativeUiAction::SetWaveformCursorPrecise { position_nanos } => {
            controller.set_waveform_cursor_nanos(position_nanos)
        }
        NativeUiAction::SetWaveformCursor { position_milli } => {
            controller.set_waveform_cursor_milli(position_milli)
        }
        NativeUiAction::BeginWaveformSelectionAt { anchor_micros } => {
            controller.start_selection_drag(anchor_micros.min(1_000_000) as f32 / 1_000_000.0);
            controller.focus_waveform_context();
        }
        NativeUiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        } => controller.set_waveform_selection_range_micros_with_drag_policy(
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        ),
        NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_micros,
            end_micros,
        } => controller.set_waveform_selection_range_micros_smart_scale(start_micros, end_micros),
        NativeUiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        } => controller.set_waveform_edit_selection_range_micros_with_edge_policy(
            start_micros,
            end_micros,
            preserve_view_edge,
        ),
        NativeUiAction::SetWaveformEditFadeInEnd { position_micros } => {
            controller.set_waveform_edit_fade_in_end_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
            controller.set_waveform_edit_fade_in_mute_start_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeInCurve { curve_milli } => {
            controller.set_waveform_edit_fade_in_curve_milli(curve_milli)
        }
        NativeUiAction::SetWaveformEditFadeOutStart { position_micros } => {
            controller.set_waveform_edit_fade_out_start_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
            controller.set_waveform_edit_fade_out_mute_end_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
            controller.set_waveform_edit_fade_out_curve_milli(curve_milli)
        }
        NativeUiAction::FinishWaveformEditFadeDrag => controller.finish_waveform_edit_fade_drag(),
        NativeUiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        } => {
            let Some(bounds) = controller.ui.waveform.selection else {
                return Ok(());
            };
            controller.start_selection_drag_payload(
                bounds,
                native_drag_point(pointer_x, pointer_y),
                true,
            );
            controller.ui.drag.origin_source = Some(DragSource::Waveform);
        }
        NativeUiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            over_browser_list,
            shift_down,
            alt_down,
        } => controller.update_active_drag(
            native_drag_point(pointer_x, pointer_y),
            DragSource::Browser,
            if over_browser_list {
                DragTarget::BrowserList
            } else {
                DragTarget::None
            },
            shift_down,
            alt_down,
        ),
        NativeUiAction::FinishWaveformSelectionDrag => controller.finish_active_drag(),
        NativeUiAction::FinishWaveformSelectionRangeDrag => controller.finish_selection_drag(),
        NativeUiAction::FinishWaveformSelectionSmartScaleDrag => controller.finish_selection_drag(),
        NativeUiAction::FinishWaveformEditSelectionDrag => controller.finish_edit_selection_drag(),
        NativeUiAction::SaveWaveformSelectionToBrowser => {
            controller.save_waveform_selection_or_slices_to_browser_action(true)
        }
        NativeUiAction::SaveWaveformSelectionToBrowserWithKeep2 => controller
            .save_waveform_selection_or_slices_to_browser_action_with_tag(
                true,
                Some(crate::sample_sources::Rating::new(2)),
            ),
        NativeUiAction::CommitWaveformEditFades => {
            let _ = controller.commit_edit_selection_fades();
        }
        NativeUiAction::DetectWaveformSilenceSlices => {
            controller.detect_waveform_silence_slices_action();
        }
        NativeUiAction::DetectWaveformExactDuplicateSlices => {
            controller.detect_waveform_exact_duplicate_slices_action();
        }
        NativeUiAction::CleanWaveformExactDuplicateSlices => {
            let _ = controller.request_destructive_selection_edit(
                DestructiveSelectionEdit::CleanExactDuplicateBeats,
            );
        }
        NativeUiAction::ClearWaveformSelection => controller.clear_waveform_selection_with_focus(),
        NativeUiAction::ClearWaveformEditSelection => {
            controller.clear_waveform_edit_selection_with_focus()
        }
        NativeUiAction::ClearWaveformSelections => controller.clear_waveform_marks_with_focus(),
        NativeUiAction::SetWaveformViewCenter { center_micros } => {
            controller.scroll_waveform_view_with_focus(center_micros)
        }
        NativeUiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros,
        } => {
            controller.zoom_waveform_steps_from_ui_with_anchor(zoom_in, steps, anchor_ratio_micros)
        }
        NativeUiAction::ZoomWaveformToSelection => {
            controller.zoom_waveform_to_selection_with_focus()
        }
        NativeUiAction::ZoomWaveformFull => controller.zoom_waveform_full_with_focus(),
        NativeUiAction::NormalizeWaveformSelectionOrSample => {
            controller.normalize_waveform_selection_or_sample_action()
        }
        NativeUiAction::CropWaveformSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection);
        }
        NativeUiAction::CropWaveformSelectionToNewSample => {
            if let Err(err) = controller.crop_waveform_selection_to_new_sample() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        NativeUiAction::TrimWaveformSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::TrimSelection);
        }
        NativeUiAction::ReverseWaveformSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::ReverseSelection);
        }
        NativeUiAction::FadeWaveformSelectionLeftToRight => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::FadeLeftToRight);
        }
        NativeUiAction::FadeWaveformSelectionRightToLeft => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::FadeRightToLeft);
        }
        NativeUiAction::MuteWaveformSelection => {
            handle_waveform_mute_action(controller);
        }
        NativeUiAction::DeleteSelectedSliceMarkers => {
            handle_delete_selected_slice_markers(controller);
        }
        NativeUiAction::AlignWaveformStartToMarker => {
            if let Err(err) = controller.align_waveform_start_to_last_marker() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        NativeUiAction::DeleteLoadedWaveformSample => {
            if let Err(err) = controller.delete_loaded_sample_and_navigate() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        NativeUiAction::SlideWaveformSelection { delta, fine } => {
            if fine {
                controller.nudge_selection_range(delta.into(), true);
            } else {
                controller.slide_selection_range(delta.into());
            }
        }
        NativeUiAction::ToggleTransientMarkers => {
            let enabled = !controller.ui.waveform.transient_markers_enabled;
            controller.set_transient_markers_enabled(enabled);
        }
        NativeUiAction::ToggleBpmSnap => toggle_bpm_snap(controller),
        action => return Err(action),
    }
    Ok(())
}

/// Apply one signed whole-number BPM delta from native waveform toolbar controls.
fn adjust_waveform_bpm(controller: &mut AppController, delta: i8) {
    if delta == 0 {
        return;
    }
    let current = controller.ui.waveform.bpm_value.unwrap_or(120.0);
    let next = (current + f32::from(delta)).max(1.0);
    controller.set_bpm_value(next);
}

fn handle_delete_selected_slice_markers(controller: &mut AppController) {
    if !controller.ui.waveform.slice_mode_enabled {
        return;
    }
    if controller.loaded_waveform_slice_export_in_progress() {
        controller.set_status(
            "Wait for the current slice export to finish",
            StatusTone::Info,
        );
        return;
    }
    let removed = controller.delete_selected_slices();
    if removed > 0 {
        controller.set_status(format!("Deleted {removed} slices"), StatusTone::Info);
    } else {
        controller.set_status("Select slices to delete", StatusTone::Info);
    }
}

fn handle_waveform_mute_action(controller: &mut AppController) {
    if controller.ui.waveform.slice_mode_enabled {
        if controller.loaded_waveform_slice_export_in_progress() {
            controller.set_status(
                "Wait for the current slice export to finish",
                StatusTone::Info,
            );
            return;
        }
        let selected = controller.ui.waveform.selected_slices.len();
        if selected < 2 {
            controller.set_status("Select at least 2 slices to merge", StatusTone::Info);
        } else if controller.merge_selected_slices().is_some() {
            controller.set_status(format!("Merged {selected} slices"), StatusTone::Info);
        } else {
            controller.set_status("No slices merged", StatusTone::Info);
        }
        return;
    }
    let _ = controller.request_destructive_selection_edit(DestructiveSelectionEdit::MuteSelection);
}

fn toggle_bpm_snap(controller: &mut AppController) {
    let enabled = !controller.ui.waveform.bpm_snap_enabled;
    let previous_bpm = controller.ui.waveform.bpm_value;
    controller.set_bpm_snap_enabled(enabled);
    if enabled && previous_bpm.is_none() {
        let fallback = 142.0;
        controller.set_bpm_value(fallback);
        controller.ui.waveform.bpm_input = format!("{fallback:.0}");
    }
}

/// Convert native action pointer coordinates into controller UI points.
fn native_drag_point(pointer_x: u16, pointer_y: u16) -> UiPoint {
    UiPoint::new(f32::from(pointer_x), f32::from(pointer_y))
}
