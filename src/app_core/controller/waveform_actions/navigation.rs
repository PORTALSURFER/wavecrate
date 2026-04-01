//! Waveform navigation and viewport routing for native actions.

use super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::StatusTone;

/// Try to dispatch waveform navigation native actions.
pub(super) fn apply_waveform_navigation_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
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
        NativeUiAction::BeginWaveformCircularSlide { anchor_micros } => {
            if let Err(err) =
                controller.start_waveform_circular_slide(normalize_waveform_micros(anchor_micros))
            {
                controller.set_status(err, StatusTone::Error);
            }
        }
        NativeUiAction::UpdateWaveformCircularSlide { position_micros } => {
            controller.update_waveform_circular_slide(normalize_waveform_micros(position_micros));
        }
        NativeUiAction::FinishWaveformCircularSlide => {
            if let Err(err) = controller.finish_waveform_circular_slide() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        NativeUiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        } => controller.scroll_waveform_view_with_focus(center_micros, center_nanos),
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
        NativeUiAction::SlideWaveformSelection { delta, fine } => {
            if fine {
                controller.nudge_selection_range(delta.into(), true);
            } else {
                controller.slide_selection_range(delta.into());
            }
        }
        action => return Err(action),
    }
    Ok(())
}

fn normalize_waveform_micros(position_micros: u32) -> f32 {
    position_micros.min(1_000_000) as f32 / 1_000_000.0
}
