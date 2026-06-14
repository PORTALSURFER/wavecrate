use super::super::*;

#[test]
fn moving_browser_focus_queues_async_preview_playback() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_delta_action(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
}

#[test]
fn moving_browser_focus_preserves_multi_selection_and_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_delta_action(1);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav")]
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn shift_extension_after_keyboard_toggle_uses_preserved_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.focus_browser_delta_action(1);
    controller.focus_browser_delta_action(1);

    controller.extend_browser_selection_from_focus_action(-1);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}
