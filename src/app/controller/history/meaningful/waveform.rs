use super::*;
use crate::app::state::WaveformView;

/// Reversible waveform selection, viewport, cursor, and loop UI state.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WaveformHistorySnapshot {
    selection: Option<SelectionRange>,
    edit_selection: Option<SelectionRange>,
    view: WaveformView,
    cursor: Option<f32>,
    loop_enabled: bool,
}

pub(super) fn capture_waveform_snapshot(controller: &AppController) -> WaveformHistorySnapshot {
    WaveformHistorySnapshot {
        selection: controller
            .selection_state
            .range
            .range()
            .or(controller.ui.waveform.selection),
        edit_selection: controller
            .selection_state
            .edit_range
            .range()
            .or(controller.ui.waveform.edit_selection),
        view: controller.ui.waveform.view,
        cursor: controller.ui.waveform.cursor,
        loop_enabled: controller.ui.waveform.loop_enabled,
    }
}

pub(super) fn restore_waveform_snapshot(
    controller: &mut AppController,
    snapshot: &WaveformHistorySnapshot,
) {
    controller
        .selection_state
        .range
        .set_range(snapshot.selection);
    controller.apply_selection(snapshot.selection);
    controller
        .selection_state
        .edit_range
        .set_range(snapshot.edit_selection);
    controller.selection_state.edit_fade_drag = None;
    controller.apply_edit_selection(snapshot.edit_selection);
    controller.ui.waveform.view = snapshot.view.clamp();
    controller.ui.waveform.cursor = snapshot.cursor;
    controller.ui.waveform.loop_enabled = snapshot.loop_enabled;
}
