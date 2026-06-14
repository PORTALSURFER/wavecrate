use super::*;

#[test]
fn delete_hotkey_keeps_focus_when_file_delete_fails() {
    let temp = tempdir().unwrap();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    configure_test_trash(&mut controller, &temp);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(1);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);
    controller.confirm_active_prompt_action();

    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("b.wav"))
    );
    assert_eq!(visible_browser_paths(&mut controller).len(), 3);
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Error
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("File not found for trash")
    );
}

#[test]
fn delete_browser_samples_reports_partial_failure_and_refocuses_survivor() {
    let temp = tempdir().unwrap();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    let trash_root = configure_test_trash(&mut controller, &temp);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    let err = controller
        .delete_browser_samples(&rows)
        .expect_err("partial delete should report failure");

    assert!(!source.root.join("a.wav").exists());
    assert!(trash_root.join("a.wav").exists());
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("b.wav"), PathBuf::from("c.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Moved 1 sample to trash with 1 error")
    );
    assert_eq!(controller.ui.status.text, err);
}
