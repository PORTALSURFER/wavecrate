use super::*;

/// Clear the active playback selection when plain left press starts outside the
/// current playback selection body.
///
/// The runtime still arms playback-selection drag state from the same click so
/// dragging past click slop immediately starts a fresh selection from the press
/// point while release-without-drag seeks from that point.
pub(super) fn waveform_new_selection_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    command: bool,
    alt: bool,
    shift: bool,
) -> Option<UiAction> {
    if command || alt || shift || !layout.waveform_plot.contains(point) {
        return None;
    }
    if model.waveform.selection_milli.is_none()
        || model.waveform.edit_selection_milli.is_some()
        || waveform_selection_contains_point(layout, model, point)
        || waveform_edit_selection_contains_point(layout, model, point)
    {
        return None;
    }
    Some(UiAction::ClearWaveformSelection)
}

/// Resolve outside-click deselection for one plain left-click in the waveform plot.
pub(super) fn waveform_clear_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    command: bool,
    alt: bool,
    shift: bool,
) -> Option<UiAction> {
    if command || alt || shift || !layout.waveform_plot.contains(point) {
        return None;
    }
    let clear_edit = model.waveform.edit_selection_milli.is_some()
        && !waveform_edit_selection_contains_point(layout, model, point);
    let clear_playback = model.waveform.selection_milli.is_some()
        && !waveform_selection_contains_point(layout, model, point);
    match (clear_edit, clear_playback) {
        (true, true) => Some(UiAction::ClearWaveformSelections),
        (true, false) => Some(UiAction::ClearWaveformEditSelection),
        (false, true) => Some(UiAction::ClearWaveformSelection),
        (false, false) => None,
    }
}
