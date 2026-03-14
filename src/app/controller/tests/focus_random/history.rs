use super::*;

#[test]
fn focus_history_hotkeys_are_registered() {
    let previous = hotkeys::iter_actions()
        .find(|a| a.id == "focus-history-previous")
        .expect("focus-history-previous hotkey");
    assert_eq!(previous.label, "Previous focused sample");
    assert_eq!(previous.gesture.first.key, KeyCode::ArrowLeft);
    assert!(!previous.gesture.first.shift);
    assert!(!previous.gesture.first.command);
    assert!(!previous.gesture.first.alt);
    assert!(previous.gesture.chord.is_none());

    let next = hotkeys::iter_actions()
        .find(|a| a.id == "focus-history-next")
        .expect("focus-history-next hotkey");
    assert_eq!(next.label, "Next focused sample");
    assert_eq!(next.gesture.first.key, KeyCode::ArrowRight);
    assert!(!next.gesture.first.shift);
    assert!(!next.gesture.first.command);
    assert!(!next.gesture.first.alt);
    assert!(next.gesture.chord.is_none());
}

#[test]
fn focus_history_steps_backward_and_forward() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("c.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    controller.focus_browser_row(1);
    controller.focus_browser_row(2);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("c.wav"))
    );

    controller.focus_previous_sample_history();
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("b.wav"))
    );

    controller.focus_previous_sample_history();
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("a.wav"))
    );

    controller.focus_next_sample_history();
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("b.wav"))
    );
}
