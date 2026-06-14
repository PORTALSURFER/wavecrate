use super::*;
use crate::app_core::actions::NativeOptionsAction;

#[test]
fn apply_ui_action_routes_transport_and_options_cases() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::ToggleLoopPlayback,
    ));
    assert!(controller.ui.waveform.loop_enabled);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::OpenOptionsMenu,
    ));
    assert!(controller.ui.options_panel.open);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetInputMonitoringEnabled { enabled: false },
    ));
    assert!(!controller.ui.controls.input_monitoring_enabled);
}

#[test]
fn apply_ui_loop_lock_cycles_locked_loop_override() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Options(NativeOptionsAction::ToggleLoopLock));
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(controller.ui.waveform.loop_enabled);

    controller.apply_ui_action(NativeUiAction::Options(NativeOptionsAction::ToggleLoopLock));
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(!controller.ui.waveform.loop_enabled);
}
