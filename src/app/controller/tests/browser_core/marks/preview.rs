use super::*;

#[test]
fn focused_browser_mark_advances_and_previews_next_sample() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.browser.selection.commit_focus_pending);

    wait_for_waveform_image(&mut controller, Path::new("two.wav"));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_some());
}

#[test]
fn focused_browser_mark_advances_and_previews_next_sample_repeatedly() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    wait_for_waveform_image(&mut controller, Path::new("two.wav"));

    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
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

    wait_for_waveform_image(&mut controller, Path::new("three.wav"));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert!(controller.ui.waveform.image.is_some());
}

#[test]
fn unmarking_focused_marked_row_under_marked_filter_refocuses_next_visible_row() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav")]
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn unmarking_focused_marked_row_under_marked_filter_previews_replacement_sample() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();
    controller.focus_browser_row_only(0);

    controller.toggle_browser_sample_mark();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.browser.selection.commit_focus_pending);
}
