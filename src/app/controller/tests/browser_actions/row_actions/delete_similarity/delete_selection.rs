use super::*;

#[test]
fn delete_actions_apply_to_all_selected_rows() {
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
    let rows = controller.action_rows_from_primary(0);

    controller.delete_browser_samples(&rows).unwrap();

    assert_eq!(controller.wav_entries_len(), 0);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(!source.root.join("three.wav").exists());
    assert!(trash_root.join("one.wav").exists());
    assert!(trash_root.join("two.wav").exists());
    assert!(trash_root.join("three.wav").exists());
}

#[test]
fn delete_active_browser_selection_includes_hidden_selected_paths() {
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
    controller.set_browser_search(String::from("one"));

    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("one.wav")]
    );

    assert!(controller.delete_active_browser_selection());

    assert_eq!(controller.wav_entries_len(), 1);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(source.root.join("three.wav").exists());
    assert!(trash_root.join("one.wav").exists());
    assert!(trash_root.join("two.wav").exists());
}

#[test]
fn delete_browser_samples_requires_configured_trash_folder() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);

    controller.delete_browser_samples(&[0]).unwrap();

    assert_eq!(controller.wav_entries_len(), 1);
    assert!(source.root.join("one.wav").exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert_eq!(
        controller.ui.status.text,
        "Set a trash folder first to auto-trash samples"
    );
}
