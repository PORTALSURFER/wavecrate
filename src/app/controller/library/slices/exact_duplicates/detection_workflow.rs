use super::{super::*, WaveformSliceBatchProfile, build_duplicate_cleanup_state, inputs};
use wavecrate_analysis::detect_exact_duplicate_window_ranges;

impl AppController {
    /// Detect near-duplicate hit windows across the loaded waveform using the current selection size.
    pub(crate) fn detect_waveform_exact_duplicate_slices_from_selection(
        &mut self,
    ) -> Result<usize, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        let input = inputs::duplicate_detection_input(self)?;
        let detection = detect_exact_duplicate_window_ranges(
            input.samples.as_ref(),
            input.channels,
            input.sample_rate,
            input.scan.window_frames,
            input.scan.anchor_start_frame,
            &input.transient_frames,
        )?;
        if detection.duplicate_windows.is_empty() {
            self.clear_waveform_slices();
            self.set_status(
                "No near-duplicate windows found for the current selection size",
                StatusTone::Info,
            );
            return Ok(0);
        }

        self.install_duplicate_cleanup_detection(
            &detection.duplicate_windows,
            detection.duplicate_group_count,
            input.total_frames,
        );
        Ok(self.ui.waveform.slices.len())
    }

    /// Run duplicate window detection and surface any failure via status UI.
    pub(crate) fn detect_waveform_exact_duplicate_slices_action(&mut self) {
        if self.loaded_waveform_slice_export_in_progress() {
            self.set_status(
                "Wait for the current slice export to finish",
                StatusTone::Info,
            );
            self.focus_waveform_context();
            return;
        }
        if let Err(err) = self.detect_waveform_exact_duplicate_slices_from_selection() {
            self.set_error_status(err);
        }
        self.focus_waveform_context();
    }

    fn install_duplicate_cleanup_detection(
        &mut self,
        windows: &[wavecrate_analysis::DetectedDuplicateWindow],
        duplicate_group_count: usize,
        total_frames: usize,
    ) {
        self.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
        self.ui.waveform.slice_mode_enabled = true;
        self.ui.waveform.selected_slices.clear();
        self.ui.waveform.duplicate_cleanup = Some(build_duplicate_cleanup_state(
            windows,
            duplicate_group_count,
            total_frames,
        ));
        self.sync_duplicate_cleanup_previews();
        self.start_slice_review();
    }
}
