use super::*;

/// Edit fade-out bottom-handle updates should keep existing crossfade handles fixed.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_preserves_crossfade_handles() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_in_mute(0.25)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(0.25);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_in = updated.fade_in().expect("fade-in should remain");
    let fade_out = updated.fade_out().expect("fade-out should remain");
    let fade_in_end = updated.start() + (updated.width() * fade_in.length);
    let fade_in_outer_start = updated.start() - (updated.width() * fade_in.mute);
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    let fade_out_outer_end = updated.end() + (updated.width() * fade_out.mute);
    assert!((updated.start() - 0.2).abs() < 0.001);
    assert!((updated.end() - 0.7).abs() < 0.001);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_in_outer_start - 0.1).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.001);
}

/// Edit fade-in bottom-handle updates should keep existing crossfade handles fixed.
#[test]
fn set_waveform_edit_fade_in_mute_start_milli_preserves_crossfade_handles() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_in_mute(0.25)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(0.25);
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
    let fade_in_outer_start = updated.start() - (updated.width() * fade_in.mute);
    let fade_out_start = updated.end() - (updated.width() * fade_out.length);
    let fade_out_outer_end = updated.end() + (updated.width() * fade_out.mute);
    assert!((updated.start() - 0.1).abs() < 0.001);
    assert!((updated.end() - 0.6).abs() < 0.001);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_in_outer_start - 0.1).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.001);
}

/// Resizing the opposite edge should keep a sample-edge fade-in silence handle pinned.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_keeps_left_crossfade_pinned_to_sample_edge() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_in_mute(0.5)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_fade_out_mute_end_milli(700);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("updated edit selection");
    let fade_in = updated.fade_in().expect("fade-in should remain");
    let fade_in_outer_start = updated.start() - (updated.width() * fade_in.mute);
    assert!(fade_in_outer_start.abs() < 0.000_001);
}

/// Repeated opposite-edge updates should keep a sample-edge silence handle exactly pinned.
#[test]
fn set_waveform_edit_fade_out_mute_end_milli_keeps_left_crossfade_pinned_across_wiggles() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.3)
        .with_fade_in_mute(0.5)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    for position in [700, 690, 710, 705, 700] {
        controller.set_waveform_edit_fade_out_mute_end_milli(position);
        let updated = controller
            .ui
            .waveform
            .edit_selection
            .expect("updated edit selection");
        let fade_in = updated.fade_in().expect("fade-in should remain");
        let fade_in_outer_start = updated.start() - (updated.width() * fade_in.mute);
        assert!(
            fade_in_outer_start.abs() < 0.000_001,
            "left silence handle drifted to {fade_in_outer_start}"
        );
    }
}
