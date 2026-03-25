use super::super::*;
use crate::app::controller::test_support;
use std::path::PathBuf;

#[test]
fn selection_duration_label_uses_loaded_audio() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("clip.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    let label = controller.selection_duration_label(SelectionRange::new(0.25, 0.75));
    assert_eq!(label.as_deref(), Some("2.00 s"));
}

#[test]
fn selection_duration_label_is_absent_without_audio() {
    let (controller, _) = test_support::dummy_controller();
    let label = controller.selection_duration_label(SelectionRange::new(0.0, 1.0));
    assert!(label.is_none());
}

#[test]
fn playhead_progress_updates_position_without_play_state() {
    let (mut controller, _source) = test_support::dummy_controller();

    controller.update_playhead_from_progress(Some(0.42), false);

    assert!(controller.ui.waveform.playhead.visible);
    assert!((controller.ui.waveform.playhead.position - 0.42).abs() < 0.0001);
}

#[test]
fn playhead_progress_completion_hides_playhead() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.playhead.active_span_end = Some(1.0);

    controller.update_playhead_from_progress(Some(0.9995), false);

    assert!(!controller.ui.waveform.playhead.visible);
    assert!(controller.ui.waveform.playhead.active_span_end.is_none());
}

#[test]
fn normalized_from_milli_clamps_bounds() {
    assert_eq!(waveform_actions::normalized_from_milli(0), 0.0);
    assert_eq!(waveform_actions::normalized_from_milli(455), 0.455);
    assert_eq!(waveform_actions::normalized_from_milli(2000), 1.0);
}

#[test]
fn selection_range_from_milli_clamps_and_orders_bounds() {
    let range = waveform_actions::selection_range_from_milli(750, 250);
    assert_eq!(range.start(), 0.25);
    assert_eq!(range.end(), 0.75);

    let range = waveform_actions::selection_range_from_milli(2000, 0);
    assert_eq!(range.start(), 0.0);
    assert_eq!(range.end(), 1.0);
}

#[test]
fn zoom_steps_from_ui_clamps_to_at_least_one() {
    assert_eq!(waveform_actions::zoom_steps_from_ui(0), 1);
    assert_eq!(waveform_actions::zoom_steps_from_ui(1), 1);
    assert_eq!(waveform_actions::zoom_steps_from_ui(12), 12);
}
