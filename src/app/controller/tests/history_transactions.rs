use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use super::super::*;
use crate::app::controller::state::selection::CompareAnchorSample;
use crate::app::state::CompareAnchorState;
use std::path::PathBuf;

fn visible_browser_paths(controller: &mut AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}

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

#[test]
fn undo_redo_folder_flattened_view_transaction() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("root.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("sub/child.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("sub")).unwrap();
    controller.refresh_folder_browser_for_tests();

    controller.replace_folder_selection(0);
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );

    controller.toggle_folder_flattened_view();
    assert!(controller.ui.sources.folders.flattened_view);
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("root.wav"), PathBuf::from("sub/child.wav")]
    );

    controller.undo();
    assert!(!controller.ui.sources.folders.flattened_view);
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );

    controller.redo();
    assert!(controller.ui.sources.folders.flattened_view);
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("root.wav"), PathBuf::from("sub/child.wav")]
    );
}

#[test]
fn meaningful_ui_undo_keeps_compare_anchor_transient_state() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("current.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.set_compare_anchor_from_focused_browser_sample();
    let expected_sample = CompareAnchorSample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
    };
    let expected_ui = CompareAnchorState {
        source_id: source.id,
        relative_path: PathBuf::from("anchor.wav"),
        label: String::from("anchor"),
    };

    controller.focus_browser_row_only(1);
    controller.toggle_browser_row_selection(1);

    controller.undo();
    assert_eq!(controller.sample_view.wav.compare_anchor, Some(expected_sample.clone()));
    assert_eq!(controller.ui.compare_anchor, Some(expected_ui.clone()));
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("anchor")
    );

    controller.redo();
    assert_eq!(controller.sample_view.wav.compare_anchor, Some(expected_sample));
    assert_eq!(controller.ui.compare_anchor, Some(expected_ui));
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("anchor")
    );
}
