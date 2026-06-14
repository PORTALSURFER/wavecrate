use super::{super::*, snapping::snap_to_transient};

/// Begin one new edit-selection drag from the exact pointer anchor.
///
/// The initial anchor must remain under the pointer for predictable destructive
/// edit painting. Transient snapping still applies once the drag extends.
pub(crate) fn start_edit_selection_drag(controller: &mut AppController, position: f32) {
    let _ = controller.commit_edit_selection_fades();
    waveform_actions::clear_edit_fade_drag(controller);
    controller.begin_edit_selection_undo("Edit selection");
    let start = position.clamp(0.0, 1.0);
    let range = controller.selection_state.edit_range.begin_new(start);
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn update_edit_selection_drag(
    controller: &mut AppController,
    position: f32,
    snap_override: bool,
) {
    let range = if snap_override {
        controller.selection_state.edit_range.update_drag(position)
    } else {
        let snapped = snap_to_transient(controller, position).unwrap_or(position);
        controller.selection_state.edit_range.update_drag(snapped)
    };
    if let Some(range) = range {
        controller.apply_edit_selection(Some(range));
    } else if controller.selection_state.edit_range.range().is_none() {
        controller.apply_edit_selection(None);
    }
}

pub(crate) fn finish_edit_selection_drag(controller: &mut AppController) {
    controller.selection_state.edit_range.finish_drag();
    controller.commit_edit_selection_undo();
}

pub(crate) fn set_edit_selection_range(controller: &mut AppController, range: SelectionRange) {
    waveform_actions::clear_edit_fade_drag(controller);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn is_edit_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.edit_range.is_dragging()
}

pub(crate) fn clear_edit_selection(controller: &mut AppController) {
    let before = controller
        .selection_state
        .edit_range
        .range()
        .or(controller.ui.waveform.edit_selection);
    waveform_actions::clear_edit_fade_drag(controller);
    let cleared = controller.selection_state.edit_range.clear();
    if !cleared && controller.ui.waveform.edit_selection.is_none() {
        return;
    }
    controller.selection_state.pending_edit_undo = None;
    controller.apply_edit_selection(None);
    controller.push_edit_selection_undo("Edit selection", before, None);
}
