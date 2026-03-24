use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use super::super::*;
use std::path::PathBuf;

#[test]
fn undo_redo_browser_selection_transaction() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);

    controller.toggle_browser_row_selection(1);
    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );

    controller.undo();
    assert!(controller.browser_selected_paths_snapshot().is_empty());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.redo();
    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn undo_redo_source_selection_transaction() {
    let (mut controller, source_a) = dummy_controller();
    let source_b = SampleSource::new(source_a.root.join("other"));
    std::fs::create_dir_all(&source_b.root).unwrap();
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.cache_db(&source_a).unwrap();
    controller.cache_db(&source_b).unwrap();

    controller.select_source_by_index(0);
    controller.select_source_by_index(1);
    assert_eq!(controller.selected_source_id(), Some(source_b.id.clone()));

    controller.undo();
    assert_eq!(controller.selected_source_id(), Some(source_a.id.clone()));

    controller.redo();
    assert_eq!(controller.selected_source_id(), Some(source_b.id));
}

#[test]
fn undo_redo_folder_selection_transaction() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a/one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    std::fs::create_dir_all(source.root.join("b")).unwrap();
    controller.refresh_folder_browser_for_tests();

    let folder_a = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("a"))
        .unwrap();

    controller.replace_folder_selection(folder_a);
    controller.clear_folder_selection();
    assert!(controller.selected_folder_paths().is_empty());

    controller.undo();
    assert_eq!(controller.selected_folder_paths(), vec![PathBuf::from("a")]);

    controller.redo();
    assert!(controller.selected_folder_paths().is_empty());
}
