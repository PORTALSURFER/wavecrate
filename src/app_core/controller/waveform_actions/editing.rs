//! Waveform editing, export, and slice cleanup routing for native actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::{DestructiveSelectionEdit, StatusTone};

/// Try to dispatch waveform edit and export native actions.
pub(super) fn apply_waveform_edit_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SaveWaveformSelectionToBrowser => controller
            .save_waveform_selection_or_slices_to_browser_action_with_tag(
                true,
                Some(crate::sample_sources::Rating::KEEP_1),
            ),
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
        NativeUiAction::MuteWaveformSelection => handle_waveform_mute_action(controller),
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
        action => return Err(action),
    }
    Ok(())
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
