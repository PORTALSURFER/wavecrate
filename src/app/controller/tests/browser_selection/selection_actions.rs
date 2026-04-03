use super::*;

#[test]
fn hotkey_tagging_applies_to_all_selected_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.tag_selected_left();

    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
    assert_eq!(
        controller.wav_entry(1).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
}

#[test]
fn folder_hotkey_moves_selected_samples() {
    let (mut controller, source) = dummy_controller();
    let destination = source.root.join("dest");
    std::fs::create_dir_all(&destination).unwrap();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["one.wav", "two.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    write_test_wav(&destination.join("existing.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("dest/existing.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.bind_folder_hotkey(Path::new("dest"), Some(1));
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);

    let handled = controller.apply_folder_hotkey(1, FocusContext::SampleBrowser);

    assert!(handled);
    assert!(destination.join("one.wav").exists());
    assert!(destination.join("two.wav").exists());
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(
        controller
            .wav_index_for_path(&PathBuf::from("dest/one.wav"))
            .is_some()
    );
    assert!(
        controller
            .wav_index_for_path(&PathBuf::from("dest/two.wav"))
            .is_some()
    );
}

#[test]
fn update_selection_paths_rewrites_browser_selected_paths() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.ui.browser.selection.selected_paths =
        vec![PathBuf::from("old.wav"), PathBuf::from("keep.wav")];

    controller.update_selection_paths(&source, Path::new("old.wav"), Path::new("new.wav"));

    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("new.wav"), PathBuf::from("keep.wav")]
    );
}

#[test]
fn browser_action_paths_keep_hidden_selected_members() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);

    controller.set_browser_search(String::from("one"));

    assert_eq!(
        controller.browser_action_paths_from_primary(0),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn update_cached_entry_replaces_old_path_in_lookup() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("old.wav")];

    let mut updated = sample_entry("new.wav", crate::sample_sources::Rating::NEUTRAL);
    updated.file_size = 10;
    updated.modified_ns = 7;
    controller.update_cached_entry(&source, Path::new("old.wav"), updated);

    assert!(
        controller
            .wav_index_for_path(Path::new("old.wav"))
            .is_none()
    );
    assert!(
        controller
            .wav_index_for_path(Path::new("new.wav"))
            .is_some()
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("new.wav")]
    );
}

#[test]
fn select_all_populates_visible_browser_paths() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_all_browser_rows();

    assert_eq!(controller.ui.browser.selection.selected_paths.len(), 3);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}
