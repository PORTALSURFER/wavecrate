use super::*;

#[test]
fn playhead_completion_detects_full_span_end() {
    let (controller, _source) = dummy_controller();

    assert!(!controller.playhead_completed_span_for_tests(0.5, false));
    assert!(controller.playhead_completed_span_for_tests(0.9995, false));
    assert!(!controller.playhead_completed_span_for_tests(1.0, true));
}

#[test]
fn playhead_completion_tracks_selection_end() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.waveform.playhead.active_span_end = Some(0.25);

    assert!(!controller.playhead_completed_span_for_tests(0.2, false));
    assert!(controller.playhead_completed_span_for_tests(0.251, false));
}

#[test]
fn hiding_playhead_clears_span_target() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.active_span_end = Some(0.4);

    controller.hide_waveform_playhead_for_tests();

    assert!(!controller.ui.waveform.playhead.visible);
    assert!(controller.ui.waveform.playhead.active_span_end.is_none());
}

#[test]
fn last_start_marker_clamps_and_resets() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marker.wav");

    controller.record_play_start(-0.25);
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));

    controller.record_play_start(0.75);
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.75));

    controller.clear_waveform_view();
    assert!(controller.ui.waveform.last_start_marker.is_none());
}

#[test]
fn selecting_new_sample_clears_last_start_marker() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_wav_by_path(Path::new("a.wav"));
    controller.record_play_start(0.25);
    controller.select_wav_by_path(Path::new("b.wav"));

    assert!(controller.ui.waveform.last_start_marker.is_none());
}
