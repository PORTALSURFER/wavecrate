use super::*;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;

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

#[test]
fn focus_history_hidden_target_stays_preview_only() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("c.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);

    controller.focus_browser_row(0);
    controller.focus_browser_row(1);
    controller.focus_browser_row(2);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;
    controller.runtime.pending_similarity_refresh = None;

    controller.set_browser_search("c");
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.focus_previous_sample_history();

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("b.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert!(controller.runtime.pending_similarity_refresh.is_none());
    assert_eq!(controller.ui.browser.selection.selected_visible, None);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        None
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("b.wav"))
    );
    assert!(controller.ui.browser.selection.commit_focus_pending);
    assert_eq!(controller.history.focus_history.entries.len(), 3);
    assert_eq!(controller.history.focus_history.cursor, Some(1));
}
