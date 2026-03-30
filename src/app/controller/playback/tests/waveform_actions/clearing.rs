use super::*;

/// Clearing edit selection via native helper should clear edit state and preserve focus.
#[test]
fn clear_waveform_edit_selection_with_focus_clears_edit_selection() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller
        .selection_state
        .edit_range
        .set_range(Some(SelectionRange::new(0.1, 0.4)));
    controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.1, 0.4));

    controller.clear_waveform_edit_selection_with_focus();

    assert!(controller.selection_state.edit_range.range().is_none());
    assert!(controller.ui.waveform.edit_selection.is_none());
}

/// Clearing both waveform mark types should leave the waveform panel focused and empty.
#[test]
fn clear_waveform_marks_with_focus_clears_playback_and_edit_selection() {
    let (mut controller, _source) = test_support::dummy_controller();
    let playback = SelectionRange::new(0.2, 0.6);
    let edit = SelectionRange::new(0.3, 0.5);
    controller.selection_state.range.set_range(Some(playback));
    controller.ui.waveform.selection = Some(playback);
    controller.selection_state.edit_range.set_range(Some(edit));
    controller.ui.waveform.edit_selection = Some(edit);

    controller.clear_waveform_marks_with_focus();

    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.selection_state.edit_range.range().is_none());
    assert!(controller.ui.waveform.edit_selection.is_none());
}

#[test]
fn apply_selection_updates_persisted_bpm_grid_origin_and_clear_preserves_it() {
    let (mut controller, _source) = test_support::dummy_controller();
    let selection = SelectionRange::new(0.31, 0.56);

    controller.apply_selection(Some(selection));
    controller.clear_waveform_selection_with_focus();

    assert!((controller.ui.waveform.last_bpm_grid_origin - 0.31).abs() < 1.0e-6);
    assert!(controller.ui.waveform.selection.is_none());
}
