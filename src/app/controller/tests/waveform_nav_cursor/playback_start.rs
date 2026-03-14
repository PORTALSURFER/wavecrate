use super::*;

#[test]
fn replay_from_last_start_requeues_pending_playback() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marker.wav");
    controller.select_wav_by_path(Path::new("marker.wav"));
    controller.record_play_start(0.42);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.1;

    let handled = controller.replay_from_last_start();
    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(0.42));
}

#[test]
fn play_from_start_requeues_zero_position_without_selection() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "start.wav");
    controller.select_wav_by_path(Path::new("start.wav"));
    controller.record_play_start(0.42);
    controller.ui.waveform.cursor = Some(0.25);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.6;

    let handled = controller.play_from_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(0.0));
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));
}

#[test]
fn play_from_start_prefers_active_play_selection_start() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marked.wav");
    controller.select_wav_by_path(Path::new("marked.wav"));
    let selection = crate::selection::SelectionRange::new(0.25, 0.6);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);
    controller.ui.waveform.cursor = Some(0.1);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.8;

    let handled = controller.play_from_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(selection.start()));
    assert_eq!(
        controller.ui.waveform.last_start_marker,
        Some(selection.start())
    );
}

#[test]
fn replay_from_last_start_falls_back_to_cursor() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marker.wav");
    controller.select_wav_by_path(Path::new("marker.wav"));
    controller.ui.waveform.cursor = Some(0.25);
    controller.ui.waveform.last_start_marker = None;

    let handled = controller.replay_from_last_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(0.25));
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.25));
}

#[test]
fn play_from_current_playhead_prefers_visible_playhead_position() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "playhead.wav");
    controller.select_wav_by_path(Path::new("playhead.wav"));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.58;
    controller.ui.waveform.cursor = Some(0.33);
    controller.ui.waveform.last_start_marker = Some(0.1);

    let handled = controller.play_from_current_playhead();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(0.58));
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.58));
}
