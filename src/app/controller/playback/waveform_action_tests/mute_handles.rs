use super::*;

/// Edit fade-in bottom-handle updates should resize the selection and keep fade end fixed.
#[test]
fn set_waveform_edit_fade_in_mute_start_milli_resizes_selection_start() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_in_mute_start_milli(100);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.1).abs() < 0.001);
    assert!((updated.end() - 0.6).abs() < 0.001);
    let fade_in = updated.fade_in();
    assert!(fade_in.is_some());
    let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_in.length - 0.4).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!(fade_in.mute.abs() < 0.001);
    let fade_in_end = updated.start() + (updated.width() * fade_in.length);
    assert!((fade_in_end - 0.3).abs() < 0.001);
}

/// Edit fade-out bottom-handle updates should resize the selection and keep fade start fixed.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_resizes_selection_end() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.2).abs() < 0.001);
    assert!((updated.end() - 0.7).abs() < 0.001);
    let fade_out = updated.fade_out();
    assert!(fade_out.is_some());
    let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_out.length - 0.4).abs() < 0.001);
    assert!((fade_out.curve - 0.2).abs() < 0.001);
    assert!(fade_out.mute.abs() < 0.001);
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    assert!((fade_out_start - 0.5).abs() < 0.001);
}

/// Edit fade-out bottom-handle updates should keep the opposite fade-in boundary fixed.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_preserves_fade_in_boundary() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(800);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_in = updated.fade_in().expect("fade-in should remain");
    let fade_out = updated.fade_out().expect("fade-out should remain");
    let fade_in_end = updated.start() + (updated.width() * fade_in.length);
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_in.curve - 0.3).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

/// Edit fade-in bottom-handle updates should keep the opposite fade-out boundary fixed.
#[test]
fn set_waveform_edit_fade_in_mute_start_milli_preserves_fade_out_boundary() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_in_mute_start_milli(100);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_in = updated.fade_in().expect("fade-in should remain");
    let fade_out = updated.fade_out().expect("fade-out should remain");
    let fade_in_end = updated.start() + (updated.width() * fade_in.length);
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_in.curve - 0.3).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}
