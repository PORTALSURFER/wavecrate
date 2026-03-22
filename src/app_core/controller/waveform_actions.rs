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
        NativeUiAction::SetTransientSnapEnabled { enabled } => {
            controller.set_transient_snap_enabled(enabled)
        }
        NativeUiAction::SetTransientMarkersEnabled { enabled } => {
            controller.set_transient_markers_enabled(enabled)
        }
        NativeUiAction::SetSliceModeEnabled { enabled } => {
            controller.set_slice_mode_enabled(enabled)
        }
        NativeUiAction::SeekWaveform { position_milli } => {
            controller.queue_waveform_seek_milli(position_milli)
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
            preserve_view_edge,
        } => controller.set_waveform_selection_range_micros_with_edge_policy(
            start_micros,
            end_micros,
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
        NativeUiAction::FinishWaveformSelectionSmartScaleDrag => controller.finish_selection_drag(),
        NativeUiAction::SaveWaveformSelectionToBrowser => {
            controller.save_waveform_selection_or_slices_to_browser_action(true)
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

/// Convert native action pointer coordinates into controller UI points.
fn native_drag_point(pointer_x: u16, pointer_y: u16) -> UiPoint {
    UiPoint::new(f32::from(pointer_x), f32::from(pointer_y))
}
