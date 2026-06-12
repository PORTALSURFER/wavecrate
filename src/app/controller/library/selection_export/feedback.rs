use super::*;

impl AppController {
    /// Emit one optimistic submit token so UI projections can blink the selection immediately.
    pub(super) fn record_waveform_selection_export_flash(&mut self) {
        self.ui.waveform.selection_export_flash_nonce = self
            .ui
            .waveform
            .selection_export_flash_nonce
            .wrapping_add(1);
    }

    /// Emit one failure token so UI projections can repaint the selection in an error color.
    pub(crate) fn record_waveform_selection_export_failure_flash(&mut self) {
        self.ui.waveform.selection_export_failure_flash_nonce = self
            .ui
            .waveform
            .selection_export_failure_flash_nonce
            .wrapping_add(1);
    }

    pub(super) fn record_selection_export_timings(
        &self,
        action: &str,
        relative_path: &Path,
        timings: SelectionExportTimings,
    ) {
        tracing::debug!(
            "selection_export action={} path={} prepare_us={} write_us={} register_us={} total_us={}",
            action,
            relative_path.display(),
            timings.prepare.as_micros(),
            timings.write.as_micros(),
            timings.register.as_micros(),
            timings.total.as_micros()
        );
    }
}
