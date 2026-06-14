use super::*;

#[test]
fn delete_hotkey_prompts_then_applies_to_all_selected_rows() {
    let temp = tempdir().unwrap();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    let trash_root = configure_test_trash(&mut controller, &temp);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_row_selection(2);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert!(matches!(
        controller.ui.browser.pending_action,
        Some(crate::app::state::SampleBrowserActionPrompt::Delete { .. })
    ));
    assert_eq!(controller.wav_entries_len(), 3);
    assert!(source.root.join("one.wav").exists());

    controller.confirm_active_prompt_action();

    assert_eq!(controller.wav_entries_len(), 0);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(!source.root.join("three.wav").exists());
    assert!(trash_root.join("one.wav").exists());
    assert!(trash_root.join("two.wav").exists());
    assert!(trash_root.join("three.wav").exists());
}

#[test]
fn canceling_delete_hotkey_keeps_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);
    controller.cancel_active_prompt_action();

    assert!(controller.ui.browser.pending_action.is_none());
    assert_eq!(controller.wav_entries_len(), 2);
    assert!(source.root.join("one.wav").exists());
    assert!(source.root.join("two.wav").exists());
}

#[test]
fn delete_hotkey_waits_for_loading_sample() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.ui.waveform.loading = Some(PathBuf::from("one.wav"));
    controller
        .runtime
        .jobs
        .set_pending_audio(Some(PendingAudio {
            request_id: 1,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: PathBuf::from("one.wav"),
            intent: AudioLoadIntent::Selection,
        }));
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert!(source.root.join("one.wav").exists());
    assert!(matches!(
        controller.ui.browser.pending_action,
        Some(crate::app::state::SampleBrowserActionPrompt::Delete { .. })
    ));
    controller.confirm_active_prompt_action();

    assert!(source.root.join("one.wav").exists());
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.wav_entries_len(), 1);
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
    assert_eq!(
        controller.ui.status.text,
        "Wait for sample load to finish before deleting one.wav"
    );
}
