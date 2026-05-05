use super::*;

#[test]
fn apply_native_waveform_selection_range_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_selection_range(before);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 200_000,
        end_micros: 700_000,
        snap_override: false,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformSelectionRangeDrag);

    assert_eq!(controller.ui.waveform.selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.selection, Some(after));
}

#[test]
fn apply_native_waveform_edit_selection_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_edit_selection_range(before);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformEditSelectionRange {
        start_micros: 200_000,
        end_micros: 700_000,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditSelectionDrag);

    assert_eq!(controller.ui.waveform.edit_selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));
}

#[test]
fn apply_native_waveform_edit_fade_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.set_edit_selection_range(before);

    controller
        .apply_native_ui_action(NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli: 750 });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditFadeDrag);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after drag");
    assert!((updated.curve - 0.75).abs() < 0.001);

    controller.undo();
    let undone = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after undo");
    assert!((undone.curve - 0.2).abs() < 0.001);

    controller.redo();
    let redone = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after redo");
    assert!((redone.curve - 0.75).abs() < 0.001);
}

#[test]
fn clear_waveform_edit_selection_is_undoable() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_edit_selection_range(selection);

    controller.apply_native_ui_action(NativeUiAction::ClearWaveformEditSelection);
    assert!(controller.ui.waveform.edit_selection.is_none());

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
}

#[test]
fn no_op_waveform_selection_range_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_selection_range(selection);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 200_000,
        end_micros: 600_000,
        snap_override: false,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformSelectionRangeDrag);
    controller.undo();

    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}

#[test]
fn no_op_waveform_edit_fade_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.set_edit_selection_range(selection);

    controller
        .apply_native_ui_action(NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli: 200 });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditFadeDrag);
    controller.undo();

    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}
