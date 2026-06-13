use super::*;

/// Edit fade-in handle updates should set a proportional fade-in over the edit selection.
#[test]
fn set_waveform_edit_fade_in_end_milli_updates_edit_fade_in_length() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_in_end_milli(300);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let fade_in = updated.and_then(|selection| selection.fade_in());
    assert!(fade_in.is_some());
    let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_in.length - 0.25).abs() < 0.001);
}

/// Edit fade-out handle updates should set a proportional fade-out over the edit selection.
#[test]
fn set_waveform_edit_fade_out_start_milli_updates_edit_fade_out_length() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_start_milli(500);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let fade_out = updated.and_then(|selection| selection.fade_out());
    assert!(fade_out.is_some());
    let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_out.length - 0.25).abs() < 0.001);
}

/// Edit fade-in top-handle drags should push and restore the opposite top handle.
#[test]
fn set_waveform_edit_fade_in_end_milli_pushes_and_restores_fade_out() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.2)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_in_end_milli(600);
    let pushed = controller
        .ui
        .waveform
        .edit_selection
        .expect("pushed edit selection");
    assert!(pushed.fade_out().is_none());
    let pushed_fade_in = pushed.fade_in().expect("fade-in should remain");
    assert!((pushed.start() + pushed.width() * pushed_fade_in.length - 0.6).abs() < 0.001);

    controller.set_waveform_edit_fade_in_end_milli(300);
    let restored = controller
        .ui
        .waveform
        .edit_selection
        .expect("restored edit selection");
    let fade_in = restored.fade_in().expect("fade-in should remain");
    let fade_out = restored.fade_out().expect("fade-out should restore");
    let fade_in_end = restored.start() + restored.width() * fade_in.length;
    let fade_out_start = restored.end() - restored.width() * fade_out.length;
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

/// Moving the fade-out top handle after a bottom-handle collapse should keep its silence handle.
#[test]
fn set_waveform_edit_fade_out_start_milli_preserves_silence_after_bottom_handle_collapse() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(1.0);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(500);
    controller.set_waveform_edit_fade_out_start_milli(450);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_out = updated
        .fade_out()
        .expect("fade-out silence handle should remain");
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    let fade_out_outer_end = updated.end() + (updated.width() * fade_out.mute);
    assert!((updated.end() - 0.5).abs() < 0.001);
    assert!((fade_out_start - 0.45).abs() < 0.001);
    assert!(
        (fade_out_outer_end - 1.0).abs() < 0.000_001,
        "fade_out_outer_end={fade_out_outer_end}, fade_out={fade_out:?}, updated={updated:?}"
    );
}
