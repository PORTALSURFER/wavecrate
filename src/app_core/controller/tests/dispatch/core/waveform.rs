use super::*;

#[test]
fn apply_ui_action_routes_waveform_seek_and_selection_cases() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 333_000_000,
        },
    ));
    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(333_000_000)
    );

    let mut controller = controller_for_grouped_dispatch();
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt {
            anchor_micros: 125_000,
        },
    ));
    assert_waveform_selection_millis(&controller, Some((200, 800)));

    let mut controller = controller_for_grouped_dispatch();
    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange {
            start_micros: 125_000,
            end_micros: 625_000,
            preserve_view_edge: false,
        },
    ));
    assert_waveform_edit_selection_millis(&controller, Some((125, 625)));

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections,
    ));
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.ui.waveform.edit_selection.is_none());
}

#[test]
fn apply_ui_begin_waveform_selection_at_arms_drag_without_visible_selection() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_bpm_snap_enabled(true);
    controller.set_bpm_value(120.0);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt {
            anchor_micros: 5_000,
        },
    ));

    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.is_selection_dragging());
}

fn assert_waveform_selection_millis(controller: &AppController, expected: Option<(u16, u16)>) {
    let actual = controller.ui.waveform.selection.map(|range| {
        (
            (range.start() * 1000.0).round() as u16,
            (range.end() * 1000.0).round() as u16,
        )
    });
    assert_eq!(actual, expected);
}

fn assert_waveform_edit_selection_millis(controller: &AppController, expected: Option<(u16, u16)>) {
    let actual = controller.ui.waveform.edit_selection.map(|range| {
        (
            (range.start() * 1000.0).round() as u16,
            (range.end() * 1000.0).round() as u16,
        )
    });
    assert_eq!(actual, expected);
}
