//! Waveform option, BPM, and slice-review routing for UI actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::StatusTone;

/// Try to dispatch waveform option and slice-review UI actions.
pub(super) fn apply_waveform_option_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformChannelView { stereo }) => {
            let view = if stereo {
                crate::waveform::WaveformChannelView::SplitStereo
            } else {
                crate::waveform::WaveformChannelView::Mono
            };
            controller.set_waveform_channel_view(view);
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetNormalizedAuditionEnabled { enabled }) => {
            controller.set_normalized_audition_enabled(enabled)
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { delta }) => adjust_waveform_bpm(controller, delta),
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { value_tenths }) => {
            controller.set_bpm_value(f32::from(value_tenths) / 10.0);
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetBpmSnapEnabled { enabled }) => controller.set_bpm_snap_enabled(enabled),
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetRelativeBpmGridEnabled { enabled }) => {
            controller.set_relative_bpm_grid_enabled(enabled)
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetTransientSnapEnabled { enabled }) => {
            controller.set_transient_snap_enabled(enabled)
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetTransientMarkersEnabled { enabled }) => {
            controller.set_transient_markers_enabled(enabled)
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetSliceModeEnabled { enabled }) => {
            if controller.loaded_waveform_slice_export_in_progress() {
                controller.set_status(
                    "Wait for the current slice export to finish",
                    StatusTone::Info,
                );
                return Ok(());
            }
            controller.set_slice_mode_enabled(enabled)
        }
        NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformSliceSelection { index }) => {
            controller.toggle_slice_selection(index);
            controller.focus_waveform_context();
        }
        NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::AuditionWaveformDuplicateSlice { index }) => {
            controller.audition_duplicate_cleanup_preview(index);
        }
        NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { index }) => {
            if let Err(err) = controller.toggle_duplicate_cleanup_preview_exemption(index) {
                controller.set_status(err, StatusTone::Info);
            }
            controller.focus_waveform_context();
        }
        NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::MoveWaveformSliceFocus { delta }) => {
            if !controller.move_slice_review_focus(delta) {
                controller.slide_selection_range(delta.into());
            }
        }
        NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleFocusedWaveformSliceExportMark) => {
            if let Err(err) = controller.toggle_focused_slice_export_mark() {
                controller.set_status(err, StatusTone::Info);
            }
            controller.focus_waveform_context();
        }
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleBpmSnap) => toggle_bpm_snap(controller),
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleTransientMarkers) => toggle_transient_markers(controller),
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

fn toggle_transient_markers(controller: &mut AppController) {
    let enabled = !controller.ui.waveform.transient_markers_enabled;
    controller.set_transient_markers_enabled(enabled);
}

fn toggle_bpm_snap(controller: &mut AppController) {
    let enabled = !controller.ui.waveform.bpm_snap_enabled;
    let previous_bpm = controller.ui.waveform.bpm_value;
    controller.set_bpm_snap_enabled(enabled);
    if enabled && previous_bpm.is_none() {
        let fallback = 142.0;
        controller.set_bpm_value(fallback);
        controller.set_waveform_bpm_input(Some(fallback));
    }
}
