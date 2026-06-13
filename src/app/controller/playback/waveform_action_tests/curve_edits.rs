use super::*;

/// Edit fade-in curve updates should preserve length and replace only the curve.
#[test]
fn set_waveform_edit_fade_in_curve_milli_updates_edit_fade_in_curve() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_in_curve_milli(850);

    let updated = controller.ui.waveform.edit_selection;
    let fade_in = updated.and_then(|selection| selection.fade_in());
    assert!(fade_in.is_some());
    let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_in.length - 0.25).abs() < 0.001);
    assert!((fade_in.curve - 0.85).abs() < 0.001);
}

/// Edit fade-out curve updates should preserve length and replace only the curve.
#[test]
fn set_waveform_edit_fade_out_curve_milli_updates_edit_fade_out_curve() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_curve_milli(150);

    let updated = controller.ui.waveform.edit_selection;
    let fade_out = updated.and_then(|selection| selection.fade_out());
    assert!(fade_out.is_some());
    let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_out.length - 0.25).abs() < 0.001);
    assert!((fade_out.curve - 0.15).abs() < 0.001);
}
