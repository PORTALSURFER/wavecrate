use super::*;

#[test]
fn normalize_actions_apply_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    controller.normalize_browser_samples(&rows).unwrap();

    let entries = controller.wav_entries.pages.get(&0).expect("entries");
    assert!(entries.iter().all(|entry| entry.modified_ns > 0));
    assert!(entries.iter().all(|entry| entry.file_size > 0));
}

#[test]
fn delete_actions_warn_when_retained_recovery_is_processing_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("busy/one.wav", Rating::NEUTRAL),
        sample_entry("busy/two.wav", Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("busy")).unwrap();
    write_test_wav(&source.root.join("busy/one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("busy/two.wav"), &[0.0, 0.1]);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("busy"),
        }],
    });

    controller.delete_browser_samples(&[0, 1]).unwrap();

    assert_eq!(controller.wav_entries_len(), 2);
    assert!(source.root.join("busy/one.wav").exists());
    assert!(source.root.join("busy/two.wav").exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}

#[test]
fn normalize_actions_warn_when_retained_recovery_is_processing_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("busy/one.wav", Rating::NEUTRAL),
        sample_entry("busy/two.wav", Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("busy")).unwrap();
    write_test_wav(&source.root.join("busy/one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("busy/two.wav"), &[0.0, 0.1]);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("busy"),
        }],
    });

    controller.normalize_browser_samples(&[0, 1]).unwrap();

    let entries = controller.wav_entries.pages.get(&0).expect("entries");
    assert!(entries.iter().all(|entry| entry.modified_ns == 0));
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}
