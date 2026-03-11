//! Shared waveform-action selection state helpers.

use super::*;
use crate::selection::SelectionRange;

/// Return the active playback selection, preferring transient drag state when present.
pub(super) fn current_playback_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
}

/// Return the active edit selection, preferring transient drag state when present.
pub(super) fn current_edit_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .edit_range
        .range()
        .or(controller.ui.waveform.edit_selection)
}

/// Apply an edit-selection update, skipping no-op updates when waveform focus is already active.
pub(super) fn apply_edit_selection_update(
    controller: &mut AppController,
    existing_range: Option<SelectionRange>,
    next_range: SelectionRange,
) {
    if existing_range == Some(next_range) && waveform_focus_active(controller) {
        return;
    }
    controller
        .selection_state
        .edit_range
        .set_range(Some(next_range));
    controller.apply_edit_selection(Some(next_range));
    controller.focus_waveform();
}
