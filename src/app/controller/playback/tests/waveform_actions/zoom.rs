use super::*;

/// UI zoom should preserve the cursor's relative viewport position as the zoom anchor.
#[test]
fn zoom_steps_from_ui_preserves_cursor_anchor_ratio() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.8,
    };
    controller.ui.waveform.cursor = Some(0.35);

    let before = controller.ui.waveform.view;
    let cursor = f64::from(controller.ui.waveform.cursor.unwrap_or(0.0));
    let before_ratio = (cursor - before.start) / (before.end - before.start);

    controller.zoom_waveform_steps_from_ui(true, 1);

    let after = controller.ui.waveform.view;
    let after_ratio = (cursor - after.start) / (after.end - after.start);
    assert!((before_ratio - after_ratio).abs() < 1.0e-4);
}

/// Pointer-anchored UI zoom should preserve the hovered ratio across zoom steps.
#[test]
fn zoom_steps_from_ui_with_anchor_ratio_preserves_pointer_position() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.8,
    };
    controller.ui.waveform.cursor = Some(0.9);
    let anchor_ratio_micros = 250_000;
    let anchor = 0.35f64;

    controller.zoom_waveform_steps_from_ui_with_anchor(true, 1, Some(anchor_ratio_micros));

    let after = controller.ui.waveform.view;
    let after_ratio = (anchor - after.start) / (after.end - after.start);
    assert!((after_ratio - 0.25).abs() < 1.0e-6);
    assert!(
        controller
            .ui
            .waveform
            .cursor
            .is_some_and(|cursor| (f64::from(cursor) - anchor).abs() < 1.0e-6)
    );
}

/// UI zoom should initialize cursor at view center when none exists.
#[test]
fn zoom_steps_from_ui_initializes_cursor_at_view_center() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.9,
    };
    controller.ui.waveform.cursor = None;

    controller.zoom_waveform_steps_from_ui(true, 1);

    assert_eq!(controller.ui.waveform.cursor, Some(0.5));
}

#[test]
fn native_waveform_view_center_does_not_snap_back_to_visible_playhead() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.0;

    controller.apply_native_ui_action(NativeUiAction::SetWaveformViewCenter {
        center_micros: 700_000,
        center_nanos: None,
    });

    assert!((controller.ui.waveform.view.start - 0.6).abs() < 1.0e-6);
    assert!((controller.ui.waveform.view.end - 0.8).abs() < 1.0e-6);
}

#[test]
fn native_waveform_view_center_uses_precise_nanos_when_available() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.5,
        end: 0.500_000_2,
    };

    controller.apply_native_ui_action(NativeUiAction::SetWaveformViewCenter {
        center_micros: 500_000,
        center_nanos: Some(500_000_050),
    });

    assert!((controller.ui.waveform.view.start - 0.499_999_95).abs() < 1.0e-9);
    assert!((controller.ui.waveform.view.end - 0.500_000_15).abs() < 1.0e-9);
}

/// Tiny floating-point drift should not be treated as a waveform view change.
#[test]
fn waveform_view_changed_ignores_tiny_float_noise() {
    let base = crate::app::state::WaveformView {
        start: 0.25,
        end: 0.75,
    };
    let nearly_equal = crate::app::state::WaveformView {
        start: 0.25 + (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
        end: 0.75 - (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
    };
    assert!(!waveform_actions::waveform_view_changed(base, nearly_equal));
}
