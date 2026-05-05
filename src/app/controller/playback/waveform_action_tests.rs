use super::*;
use crate::app::controller::{LoadedAudio, test_support};
use std::path::PathBuf;

/// Edit-selection updates should no-op when the range is unchanged and waveform is focused.
#[test]
fn set_waveform_edit_selection_range_milli_noops_when_unchanged_and_focused() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
    let range = SelectionRange::new(0.2, 0.6);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_selection_range_milli(200, 600);

    assert_eq!(controller.selection_state.edit_range.range(), Some(range));
    assert_eq!(controller.ui.waveform.edit_selection, Some(range));
}

/// Edit-selection start-edge resize should keep the fade-in attached to the moved edge.
#[test]
fn set_waveform_edit_selection_range_milli_preserves_fade_in_on_start_resize() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_selection_range_milli(600, 100);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.1).abs() < 0.001);
    assert!((updated.end() - 0.6).abs() < 0.001);
    let fade_in = updated.fade_in();
    assert!(fade_in.is_some());
    let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_in.length - 0.2).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    let fade_in_abs = updated.width() * fade_in.length;
    assert!((fade_in_abs - 0.1).abs() < 0.001);
}

/// Edit-selection end-edge resize should keep the fade-out attached to the moved edge.
#[test]
fn set_waveform_edit_selection_range_milli_preserves_fade_out_on_end_resize() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_selection_range_milli(200, 700);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.2).abs() < 0.001);
    assert!((updated.end() - 0.7).abs() < 0.001);
    let fade_out = updated.fade_out();
    assert!(fade_out.is_some());
    let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((fade_out.length - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.2).abs() < 0.001);
    let fade_out_abs = updated.width() * fade_out.length;
    assert!((fade_out_abs - 0.1).abs() < 0.001);
}

/// Start-edge resize should shrink only the moved-side fade when the new span is too small.
#[test]
fn set_waveform_edit_selection_range_milli_shrinks_moved_side_fade_first() {
    let (mut controller, _source) = test_support::dummy_controller();
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_in(0.25, 0.2)
        .with_fade_out(0.25, 0.7);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_selection_range_milli(800, 600);

    let updated = controller.ui.waveform.edit_selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.6).abs() < 0.001);
    assert!((updated.end() - 0.8).abs() < 0.001);
    let fade_in = updated.fade_in();
    let fade_out = updated.fade_out();
    assert!(fade_in.is_some());
    assert!(fade_out.is_some());
    let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
    assert!((updated.width() * fade_in.length - 0.05).abs() < 0.001);
    assert!((updated.width() * fade_out.length - 0.15).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

/// Edit-selection translations should snap the moved range to BPM steps.
#[test]
fn set_waveform_edit_selection_range_milli_snaps_translated_range_when_bpm_snap_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.4).with_fade_in(0.25, 0.2);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.ui.waveform.edit_selection = Some(range);

    controller.set_waveform_edit_selection_range_milli(260, 460);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("translated edit selection");
    assert!((updated.start() - 0.25).abs() < 0.001);
    assert!((updated.end() - 0.45).abs() < 0.001);
    let fade_in = updated.fade_in().expect("fade-in should be preserved");
    assert!((fade_in.length - 0.25).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
}

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
