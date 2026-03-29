use super::*;

impl AppController {
    /// Return whether keyboard-first slice review is active for the waveform.
    pub(crate) fn slice_review_active(&self) -> bool {
        self.ui.waveform.slice_review.active
    }

    /// Return the currently focused review slice range, if any.
    pub(crate) fn focused_slice_review_range(&self) -> Option<SelectionRange> {
        let index = self.ui.waveform.slice_review.focused_index?;
        self.ui.waveform.slices.get(index).copied()
    }

    /// Start keyboard slice review with the first slice focused.
    pub(crate) fn start_slice_review(&mut self) -> bool {
        if self.ui.waveform.slices.is_empty() {
            self.ui.waveform.slice_review = Default::default();
            return false;
        }
        self.ui.waveform.slice_review.active = true;
        self.ui.waveform.slice_review.focused_index = Some(0);
        self.ui.waveform.slice_review.marked_indices.clear();
        self.ensure_selection_visible_in_view(self.ui.waveform.slices[0]);
        self.focus_waveform_context();
        self.set_status(self.slice_review_hint(), StatusTone::Info);
        true
    }

    /// Exit keyboard slice review while preserving any existing slice previews.
    pub(crate) fn exit_slice_review(&mut self) -> bool {
        if !self.ui.waveform.slice_review.active {
            return false;
        }
        self.ui.waveform.slice_review.active = false;
        self.ui.waveform.slice_review.focused_index = None;
        self.set_status("Exited slice review", StatusTone::Info);
        true
    }

    /// Move the focused review slice by one signed delta, clamping at the batch edges.
    pub(crate) fn move_slice_review_focus(&mut self, delta: i8) -> bool {
        if !self.ui.waveform.slice_review.active || self.ui.waveform.slices.is_empty() || delta == 0
        {
            return false;
        }
        let current = self.ui.waveform.slice_review.focused_index.unwrap_or(0);
        let last = self.ui.waveform.slices.len().saturating_sub(1);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current.saturating_add(delta as usize).min(last)
        };
        if next == current {
            self.set_status(
                if current == 0 {
                    "Already at the first slice"
                } else {
                    "Already at the last slice"
                },
                StatusTone::Info,
            );
            return true;
        }
        self.ui.waveform.slice_review.focused_index = Some(next);
        self.ensure_selection_visible_in_view(self.ui.waveform.slices[next]);
        self.focus_waveform_context();
        self.set_status(self.slice_review_hint(), StatusTone::Info);
        true
    }

    /// Toggle export marking on the currently focused review slice.
    pub(crate) fn toggle_focused_slice_export_mark(&mut self) -> Result<bool, String> {
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            return Err("Exact duplicate cleanup batches cannot be export-marked".to_string());
        }
        let index = self
            .ui
            .waveform
            .slice_review
            .focused_index
            .ok_or_else(|| "Focus a slice first".to_string())?;
        if index >= self.ui.waveform.slices.len() {
            return Err("Focus a slice first".to_string());
        }
        if let Some(position) = self
            .ui
            .waveform
            .slice_review
            .marked_indices
            .iter()
            .position(|value| *value == index)
        {
            self.ui
                .waveform
                .slice_review
                .marked_indices
                .swap_remove(position);
            self.ui.waveform.slice_review.marked_indices.sort_unstable();
            self.set_status(
                format!(
                    "Unmarked slice {} for export ({} marked)",
                    index + 1,
                    self.ui.waveform.slice_review.marked_indices.len()
                ),
                StatusTone::Info,
            );
            return Ok(false);
        }
        self.ui.waveform.slice_review.marked_indices.push(index);
        self.ui.waveform.slice_review.marked_indices.sort_unstable();
        self.set_status(
            format!(
                "Marked slice {} for export ({} marked)",
                index + 1,
                self.ui.waveform.slice_review.marked_indices.len()
            ),
            StatusTone::Info,
        );
        Ok(true)
    }

    /// Resolve the slice ranges that should be exported for the current waveform preview batch.
    pub(crate) fn waveform_slice_export_ranges(&self) -> Result<Vec<SelectionRange>, String> {
        if self.ui.waveform.slice_batch_profile == WaveformSliceBatchProfile::ExactDuplicateBeats {
            return Err("Use Clean Dups to apply exact duplicate cleanup".to_string());
        }
        if self.ui.waveform.slices.is_empty() {
            return Err("No slices to export".to_string());
        }
        if !self.ui.waveform.slice_review.marked_indices.is_empty() {
            return Ok(self
                .ui
                .waveform
                .slice_review
                .marked_indices
                .iter()
                .filter_map(|index| self.ui.waveform.slices.get(*index).copied())
                .collect());
        }
        if self.ui.waveform.slice_review.active {
            return Err("Mark slices to export first".to_string());
        }
        Ok(self.ui.waveform.slices.clone())
    }

    pub(super) fn refresh_slice_review_state(&mut self) {
        if self.ui.waveform.slices.is_empty() {
            self.ui.waveform.slice_review = Default::default();
            return;
        }
        let max_index = self.ui.waveform.slices.len().saturating_sub(1);
        self.ui
            .waveform
            .slice_review
            .marked_indices
            .retain(|index| *index <= max_index);
        self.ui.waveform.slice_review.marked_indices.sort_unstable();
        self.ui.waveform.slice_review.marked_indices.dedup();
        if self.ui.waveform.slice_review.active {
            let focused = self.ui.waveform.slice_review.focused_index.unwrap_or(0);
            self.ui.waveform.slice_review.focused_index = Some(focused.min(max_index));
        } else {
            self.ui.waveform.slice_review.focused_index = None;
        }
    }

    pub(super) fn slice_review_hint(&self) -> String {
        let total = self.ui.waveform.slices.len();
        let focused = self
            .ui
            .waveform
            .slice_review
            .focused_index
            .map(|index| index + 1)
            .unwrap_or(1);
        match self.ui.waveform.slice_batch_profile {
            WaveformSliceBatchProfile::ExactDuplicateBeats => format!(
                "Cleanup {focused}/{total}. Left/Right review, Space audition, Shift+D keep, M merge, Clean Dups apply."
            ),
            _ => format!(
                "Slice {focused}/{total}. Left/Right review, Space audition, A mark, E export."
            ),
        }
    }
}
