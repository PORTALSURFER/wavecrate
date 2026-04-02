use super::*;

#[test]
fn focused_browser_mark_uses_random_preview_follow_up() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;
    controller.ui.browser.search.random_navigation_mode = true;
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("one.wav"));
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("two.wav"));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn focused_browser_mark_uses_random_preview_follow_up_repeatedly() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
        sample_entry("four.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("four.wav"), &[0.0, 0.1]);
    controller.settings.feature_flags.autoplay_selection = false;
    controller.ui.browser.search.random_navigation_mode = true;
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("two.wav"));
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("three.wav"));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    wait_for_waveform_image(&mut controller, Path::new("four.wav"));

    controller.toggle_browser_sample_mark();

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.history.random_history.entries.len(), 2);
    assert_eq!(controller.history.random_history.cursor, Some(1));
    assert_eq!(
        controller
            .history
            .random_history
            .entries
            .front()
            .map(|entry| entry.relative_path.as_path()),
        Some(Path::new("four.wav"))
    );
    assert_eq!(
        controller
            .history
            .random_history
            .entries
            .back()
            .map(|entry| entry.relative_path.as_path()),
        Some(Path::new("one.wav"))
    );

    wait_for_waveform_image(&mut controller, Path::new("one.wav"));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
}

#[test]
fn marked_filter_mark_review_uses_random_follow_up_when_random_mode_is_enabled() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(2);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();
    controller.ui.browser.search.random_navigation_mode = true;
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("one.wav"));
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("two.wav"));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav"), PathBuf::from("three.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("three.wav"))
    );
}
