use super::*;

/// Collapsing the fade-out length with the bottom handle should not collapse its silence handle.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_preserves_crossfade_when_fade_collapses() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(1.0);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(500);

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
    assert!((updated.start() - 0.2).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 1.0).abs() < 0.001);
    assert!(fade_out.length.abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

/// Crossing the silence handle during one bottom-handle drag should not pick it up.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_does_not_pick_up_silence_during_same_drag() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(1.0);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(1000);
    controller.set_waveform_edit_fade_out_mute_end_milli(700);

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
    assert!((updated.end() - 0.7).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 1.0).abs() < 0.000_001);
}

/// Releasing after collapsing into the silence handle should commit that collapsed state.
#[test]
fn finish_waveform_edit_fade_drag_commits_silence_collapse_after_bottom_handle_release() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(1.0);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(1000);
    controller.finish_waveform_edit_fade_drag();
    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_out = updated.fade_out().expect("fade-out should remain");
    let fade_out_outer_end = updated.end() + (updated.width() * fade_out.mute);
    assert!((updated.end() - 0.7).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.000_001);
}

/// Collapsed fade-out drags should recover the original fade while the same drag stays active.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_recovers_after_temporary_collapse() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(500);
    let collapsed = controller
        .ui
        .waveform
        .edit_selection
        .expect("collapsed edit selection");
    assert!(collapsed.fade_out().is_none());

    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let recovered = controller
        .ui
        .waveform
        .edit_selection
        .expect("recovered edit selection");
    assert!((recovered.end() - 0.7).abs() < 0.001);
    let fade_out = recovered.fade_out().expect("fade-out should recover");
    assert!((fade_out.length - 0.4).abs() < 0.001);
    assert!((fade_out.curve - 0.2).abs() < 0.001);
    let fade_out_start = recovered.end() - (recovered.width() * fade_out.length);
    assert!((fade_out_start - 0.5).abs() < 0.001);
}

/// Releasing a collapsed fade drag should keep the fade removed for the next gesture.
#[test]
fn finish_waveform_edit_fade_drag_commits_collapsed_fade_removal() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(500);
    controller.finish_waveform_edit_fade_drag();
    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let finished = controller
        .ui
        .waveform
        .edit_selection
        .expect("finished edit selection");
    assert!(finished.fade_out().is_none());
    assert!((finished.end() - 0.5).abs() < 0.001);
}
