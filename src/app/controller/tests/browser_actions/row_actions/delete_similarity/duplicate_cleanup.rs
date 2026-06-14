use super::*;

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
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
        sample_entry("trash.wav", Rating::NEUTRAL),
    ]);
    let trash_root = configure_test_trash(&mut controller, &temp);

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
