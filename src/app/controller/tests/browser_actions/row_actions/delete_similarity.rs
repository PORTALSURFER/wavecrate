use super::*;

#[test]
fn delete_actions_apply_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
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
}

#[test]
fn delete_active_browser_selection_includes_hidden_selected_paths() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.set_browser_search(String::from("one"));

    assert_eq!(visible_browser_paths(&mut controller), vec![PathBuf::from("one.wav")]);

    assert!(controller.delete_active_browser_selection());

    assert_eq!(controller.wav_entries_len(), 1);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(source.root.join("three.wav").exists());
}

#[test]
fn deleting_similarity_result_recomputes_filter_from_same_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[1]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("anchor.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("anchor.wav"), PathBuf::from("far.wav")]
    );
}

#[test]
fn deleting_similarity_anchor_promotes_next_best_survivor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[0]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("close.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("close.wav"), PathBuf::from("far.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("close.wav"))
    );
}

#[test]
fn entering_duplicate_cleanup_bypasses_normal_browser_filters() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("dup_a.wav", Rating::NEUTRAL),
        sample_entry("dup_b.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "dup_a.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "dup_b.wav", 0.999, 0.001);
    insert_similarity_embedding(&source, "far.wav", 0.0, 1.0);

    controller.set_browser_search(String::from("anchor"));
    controller.focus_browser_row_only(0);

    controller.enter_browser_duplicate_cleanup_mode().unwrap();

    let cleanup = controller
        .ui
        .browser
        .duplicate_cleanup
        .as_ref()
        .expect("duplicate cleanup should be active");
    assert!(cleanup.is_anchor(0));
    assert!(cleanup.is_kept(0));
    assert_eq!(cleanup.kept_indices.len(), 1);
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![
            PathBuf::from("anchor.wav"),
            PathBuf::from("dup_a.wav"),
            PathBuf::from("dup_b.wav")
        ]
    );
}

#[test]
fn confirming_duplicate_cleanup_moves_only_unkept_duplicates_to_trash() {
    let temp = tempdir().unwrap();
    let trash_root = temp.path().join("trash");
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
        sample_entry("trash.wav", Rating::NEUTRAL),
    ]);
    controller.settings.trash_folder = Some(trash_root.clone());
    controller.ui.trash_folder = Some(trash_root.clone());

    for name in ["anchor.wav", "keep.wav", "trash.wav"] {
        write_test_wav(&source.root.join(name), &[0.0, 0.1]);
        controller
            .database_for(&source)
            .unwrap()
            .upsert_file(Path::new(name), 4, 1)
            .unwrap();
    }
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "keep.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "trash.wav", 0.999, 0.001);
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.enter_browser_duplicate_cleanup_mode().unwrap();
    let keep_row = visible_browser_paths(&mut controller)
        .iter()
        .position(|path| path == Path::new("keep.wav"))
        .expect("keep duplicate should be visible");
    controller
        .toggle_browser_duplicate_cleanup_keep_for_visible_row(keep_row)
        .unwrap();

    controller.confirm_browser_duplicate_cleanup().unwrap();

    assert!(controller.ui.browser.duplicate_cleanup.is_none());
    assert!(source.root.join("anchor.wav").exists());
    assert!(source.root.join("keep.wav").exists());
    assert!(!source.root.join("trash.wav").exists());
    assert!(trash_root.join("trash.wav").exists());
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("anchor.wav"))
    );
}

#[test]
fn delete_hotkey_applies_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
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

    assert_eq!(controller.wav_entries_len(), 0);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(!source.root.join("three.wav").exists());
}

#[test]
fn delete_hotkey_keeps_focus_when_file_delete_fails() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(1);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("b.wav"))
    );
    assert_eq!(visible_browser_paths(&mut controller).len(), 3);
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Error
    );
    assert!(controller.ui.status.text.contains("Failed to delete file"));
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

#[test]
fn delete_browser_samples_reports_partial_failure_and_refocuses_survivor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    let err = controller
        .delete_browser_samples(&rows)
        .expect_err("partial delete should report failure");

    assert!(!source.root.join("a.wav").exists());
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
            .contains("Deleted 1 sample with 1 error")
    );
    assert_eq!(controller.ui.status.text, err);
}
