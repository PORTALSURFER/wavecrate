use super::*;

#[test]
fn play_from_cursor_prefers_cursor_position() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "cursor.wav");
    controller.select_wav_by_path(Path::new("cursor.wav"));
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.cursor = Some(0.33);
    controller.ui.waveform.cursor_last_navigation_at = Some(Instant::now());
    controller.ui.waveform.cursor_last_hover_at = None;
    controller.ui.waveform.last_start_marker = Some(0.1);

    let handled = controller.play_from_cursor();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(
        pending.start_override,
        controller.ui.waveform.cursor.map(f64::from)
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.33));
}

#[test]
fn play_from_cursor_ignores_hover_cursor_when_replaying() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "cursor.wav");
    controller.select_wav_by_path(Path::new("cursor.wav"));
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.cursor = Some(0.33);
    controller.ui.waveform.cursor_last_navigation_at =
        Some(Instant::now() - Duration::from_secs(5));
    controller.ui.waveform.cursor_last_hover_at = Some(Instant::now());
    controller.ui.waveform.last_start_marker = Some(0.1);

    let handled = controller.play_from_cursor();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(
        pending.start_override,
        controller.ui.waveform.last_start_marker.map(f64::from)
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.1));
}

#[test]
fn cursor_alpha_fades_before_reset() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "cursor.wav");
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.cursor = Some(0.4);
    controller.ui.waveform.cursor_last_navigation_at =
        Some(Instant::now() - Duration::from_millis(250));

    let alpha = controller.waveform_cursor_alpha(false);

    assert!((alpha - 0.5).abs() < 0.15);
    assert_eq!(controller.ui.waveform.cursor, Some(0.4));
}

#[test]
fn cursor_alpha_resets_after_idle_timeout() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "cursor.wav");
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.cursor = Some(0.4);
    controller.ui.waveform.cursor_last_navigation_at =
        Some(Instant::now() - Duration::from_millis(600));

    let alpha = controller.waveform_cursor_alpha(false);

    assert!(alpha <= f32::EPSILON);
    assert_eq!(controller.ui.waveform.cursor, Some(0.0));
}

#[test]
fn cursor_does_not_fade_when_waveform_focused() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "cursor.wav");
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.cursor = Some(0.4);
    controller.ui.waveform.cursor_last_navigation_at =
        Some(Instant::now() - Duration::from_millis(800));
    controller.ui.focus.context = FocusContext::Waveform;

    let alpha = controller.waveform_cursor_alpha(false);

    assert_eq!(alpha, 1.0);
    assert_eq!(controller.ui.waveform.cursor, Some(0.4));
}
