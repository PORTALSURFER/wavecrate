use super::HotkeysController;
use crate::app::controller::StatusTone;
use crate::app::controller::ui::hotkeys::HotkeyCommand;
use crate::app::state::DestructiveSelectionEdit;

mod delete_navigation;
mod normalization;

pub(crate) fn handle_waveform_command(
    controller: &mut HotkeysController<'_>,
    command: HotkeyCommand,
) -> bool {
    match command {
        HotkeyCommand::NormalizeWaveform => {
            controller.normalize_waveform_selection_or_sample();
            true
        }
        HotkeyCommand::AlignWaveformStartToMarker => {
            if let Err(err) = controller.align_waveform_start_to_last_marker() {
                controller.set_status(err, StatusTone::Error);
            }
            true
        }
        HotkeyCommand::CropSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection);
            true
        }
        HotkeyCommand::CropSelectionNewSample => {
            if let Err(err) = controller.crop_waveform_selection_to_new_sample() {
                controller.set_status(err, StatusTone::Error);
            }
            true
        }
        HotkeyCommand::SaveSelectionToBrowser => {
            controller.save_waveform_selection_or_slices_to_browser_action_with_tag(
                true,
                Some(crate::sample_sources::Rating::KEEP_1),
            );
            true
        }
        HotkeyCommand::TrimSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::TrimSelection);
            true
        }
        HotkeyCommand::ReverseSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::ReverseSelection);
            true
        }
        HotkeyCommand::FadeSelectionLeftToRight => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::FadeLeftToRight);
            true
        }
        HotkeyCommand::FadeSelectionRightToLeft => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::FadeRightToLeft);
            true
        }
        HotkeyCommand::DeleteSliceMarkers => {
            if controller.ui.waveform.slice_mode_enabled {
                if controller.loaded_waveform_slice_export_in_progress() {
                    controller.set_status(
                        "Wait for the current slice export to finish",
                        StatusTone::Info,
                    );
                    return true;
                }
                let removed = controller.delete_selected_slices();
                if removed > 0 {
                    controller.set_status(format!("Deleted {removed} slices"), StatusTone::Info);
                } else {
                    controller.set_status("Select slices to delete", StatusTone::Info);
                }
            }
            true
        }
        HotkeyCommand::MuteSelection => {
            if controller.ui.waveform.slice_mode_enabled {
                if controller.loaded_waveform_slice_export_in_progress() {
                    controller.set_status(
                        "Wait for the current slice export to finish",
                        StatusTone::Info,
                    );
                    return true;
                }
                let selected = controller.ui.waveform.selected_slices.len();
                if selected < 2 {
                    controller.set_status("Select at least 2 slices to merge", StatusTone::Info);
                } else if controller.merge_selected_slices().is_some() {
                    controller.set_status(format!("Merged {selected} slices"), StatusTone::Info);
                } else {
                    controller.set_status("No slices merged", StatusTone::Info);
                }
            } else {
                let _ = controller
                    .request_destructive_selection_edit(DestructiveSelectionEdit::MuteSelection);
            }
            true
        }
        HotkeyCommand::ToggleBpmSnap => {
            controller.toggle_bpm_snap();
            true
        }
        HotkeyCommand::ToggleTransientMarkers => {
            controller.toggle_transient_markers();
            true
        }
        HotkeyCommand::ZoomInSelection => {
            controller.waveform().zoom_to_selection();
            true
        }
        HotkeyCommand::SlideSelectionLeft => {
            controller.waveform().slide_selection_range(-1);
            true
        }
        HotkeyCommand::SlideSelectionRight => {
            controller.waveform().slide_selection_range(1);
            true
        }
        HotkeyCommand::NudgeSelectionLeft => {
            controller.waveform().nudge_selection_range(-1, true);
            true
        }
        HotkeyCommand::NudgeSelectionRight => {
            controller.waveform().nudge_selection_range(1, true);
            true
        }
        HotkeyCommand::ZoomOutSelection => {
            controller.waveform().zoom_out_full();
            true
        }
        HotkeyCommand::DeleteLoadedSample => {
            if let Err(err) = controller.delete_loaded_sample_and_navigate() {
                controller.set_status(err, StatusTone::Error);
            }
            true
        }
        _ => false,
    }
}

impl HotkeysController<'_> {
    fn toggle_bpm_snap(&mut self) {
        let enabled = !self.ui.waveform.bpm_snap_enabled;
        let prev_value = self.ui.waveform.bpm_value;
        self.set_bpm_snap_enabled(enabled);
        if enabled && prev_value.is_none() {
            let fallback = 142.0;
            self.set_bpm_value(fallback);
            self.ui.waveform.bpm_input = format!("{fallback:.0}");
        }
    }

    fn toggle_transient_markers(&mut self) {
        let enabled = !self.ui.waveform.transient_markers_enabled;
        self.set_transient_markers_enabled(enabled);
    }

    fn normalize_waveform_selection_or_sample(&mut self) {
        self.normalize_waveform_selection_or_sample_action();
    }
}

#[cfg(test)]
mod tests;
