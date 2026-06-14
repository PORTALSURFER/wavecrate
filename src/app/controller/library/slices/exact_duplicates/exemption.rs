use super::super::*;

impl AppController {
    /// Toggle whether one duplicate cleanup preview should be excluded from cleanup.
    pub(crate) fn toggle_duplicate_cleanup_preview_exemption(
        &mut self,
        index: usize,
    ) -> Result<bool, String> {
        let cleanup = self
            .ui
            .waveform
            .duplicate_cleanup
            .as_mut()
            .ok_or_else(|| "Run Exact Dedupe before editing duplicate cleanup".to_string())?;
        let preview = cleanup
            .previews
            .get_mut(index)
            .ok_or_else(|| "Select a duplicate cleanup preview first".to_string())?;
        preview.exempted = !preview.exempted;
        let exempted = preview.exempted;
        self.sync_duplicate_cleanup_previews();
        self.focus_duplicate_cleanup_preview(
            index.min(self.ui.waveform.slices.len().saturating_sub(1)),
        );
        let counts = self.current_duplicate_cleanup_counts();
        self.set_status(
            exemption_status(index, self.ui.waveform.slices.len(), counts, exempted),
            StatusTone::Info,
        );
        Ok(exempted)
    }
}

fn exemption_status(
    index: usize,
    slice_count: usize,
    counts: super::DuplicateCleanupCounts,
    exempted: bool,
) -> String {
    if exempted {
        format!(
            "Keeping duplicate {}/{} for now. {} marked, {} kept, {} group(s).",
            index + 1,
            slice_count,
            counts.marked_windows,
            counts.exempted_windows,
            counts.group_count
        )
    } else {
        format!(
            "Marked duplicate {}/{} for cleanup. {} marked, {} kept, {} group(s).",
            index + 1,
            slice_count,
            counts.marked_windows,
            counts.exempted_windows,
            counts.group_count
        )
    }
}
