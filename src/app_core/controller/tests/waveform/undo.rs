use super::*;

#[test]
fn apply_ui_waveform_selection_range_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_selection_range(before);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 200_000,
            end_micros: 700_000,
            snap_override: false,
            preserve_view_edge: false,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag,
    ));

    assert_eq!(controller.ui.waveform.selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.selection, Some(after));
}

#[test]
fn apply_ui_waveform_edit_selection_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_edit_selection_range(before);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange {
            start_micros: 200_000,
            end_micros: 700_000,
            preserve_view_edge: false,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag,
    ));

    assert_eq!(controller.ui.waveform.edit_selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));
}

#[test]
fn apply_ui_waveform_edit_fade_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6)
        .with_fade_out(0.25, 0.2)
        .with_fade_out_mute(0.1)
        .with_fade_out_outer_gain(0.35);
    controller.set_edit_selection_range(before);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve {
            curve_milli: 750,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve {
            curve_milli: 500,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag,
    ));

    let after = before.with_fade_out(0.25, 0.5);
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.status.text, "Undid Waveform fade");
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.undo();
    assert_eq!(controller.ui.status.text, "Nothing to undo");
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.status.text, "Redid Waveform fade");
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));
}

#[test]
fn apply_ui_waveform_edit_fade_creation_undoes_and_redoes_one_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_edit_selection_range(before);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd {
            position_micros: 300_000,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag,
    ));

    let after = before.with_fade_in(0.25, 0.5);
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));
}

#[test]
fn clear_waveform_edit_selection_is_undoable() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_edit_selection_range(selection);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection,
    ));
    assert!(controller.ui.waveform.edit_selection.is_none());

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
}

#[test]
fn no_op_waveform_selection_range_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_selection_range(selection);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 200_000,
            end_micros: 600_000,
            snap_override: false,
            preserve_view_edge: false,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag,
    ));
    controller.undo();

    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}

#[test]
fn no_op_waveform_edit_fade_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.set_edit_selection_range(selection);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve {
            curve_milli: 200,
        },
    ));
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag,
    ));
    controller.undo();

    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}
