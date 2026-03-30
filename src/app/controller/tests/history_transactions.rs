use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use super::super::*;
use crate::app::controller::state::selection::CompareAnchorSample;
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use crate::app::controller::jobs::{FileOpResult, UndoFileJob, UndoFileOpResult, UndoFileOutcome};
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

#[test]
fn restore_meaningful_ui_snapshot_recovers_browser_folder_and_waveform_context() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a/one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("a/two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    std::fs::create_dir_all(source.root.join("b")).unwrap();
    super::super::test_support::write_test_wav(&source.root.join("a/one.wav"), &[0.0, 0.1]);
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
    controller.focus_browser_row_only(1);
    controller.toggle_browser_row_selection(0);
    controller.ui.browser.selection.autoscroll = false;
    controller
        .load_waveform_for_selection(&source, std::path::Path::new("a/one.wav"))
        .unwrap();
    let waveform_selection = SelectionRange::new(0.2, 0.6);
    let edit_selection = SelectionRange::new(0.25, 0.55).with_fade_out(0.1, 0.0);
    controller.apply_selection(Some(waveform_selection));
    controller.set_edit_selection_range(edit_selection);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.7,
    };
    controller.ui.waveform.cursor = Some(0.42);
    controller.ui.waveform.loop_enabled = true;

    let snapshot = controller.capture_meaningful_ui_snapshot();

    controller.clear_folder_selection();
    controller.focus_browser_row_only(0);
    controller.clear_browser_selection();
    controller.selection_state.ctx.selected_source = None;
    controller.sample_view.wav.selected_wav = None;
    controller.sample_view.wav.loaded_wav = None;
    controller.sample_view.wav.loaded_audio = None;
    controller.set_ui_loaded_wav(None);
    controller.apply_selection(None);
    controller.apply_edit_selection(None);
    controller.ui.waveform.view = crate::app::state::WaveformView::default();
    controller.ui.waveform.cursor = None;
    controller.ui.waveform.loop_enabled = false;
    controller.ui.browser.selection.autoscroll = true;
    controller.restore_meaningful_ui_snapshot(&snapshot);

    assert_eq!(controller.selected_source_id(), Some(source.id));
    assert_eq!(controller.selected_folder_paths(), vec![PathBuf::from("a")]);
    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("a/two.wav"), PathBuf::from("a/one.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(std::path::Path::new("a/one.wav"))
    );
    assert!(!controller.ui.browser.selection.autoscroll);
    assert_eq!(controller.selection_state.range.range(), Some(waveform_selection));
    assert_eq!(controller.selection_state.edit_range.range(), Some(edit_selection));
    assert_eq!(controller.ui.waveform.selection, Some(waveform_selection));
    assert_eq!(controller.ui.waveform.edit_selection, Some(edit_selection));
    assert_eq!(
        controller.ui.waveform.view,
        crate::app::state::WaveformView {
            start: 0.1,
            end: 0.7,
        }
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.42));
    assert!(controller.ui.waveform.loop_enabled);
    assert!(
        controller
            .runtime
            .jobs
            .pending_audio()
            .as_ref()
            .is_some_and(|pending| pending.relative_path == PathBuf::from("a/one.wav"))
    );
}

#[test]
fn deferred_meaningful_ui_restore_hooks_reapply_before_and_after_snapshots() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("clip.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("clip_selection_001.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    let before_selection = SelectionRange::new(0.2, 0.6);
    let before_edit = SelectionRange::new(0.25, 0.55);
    controller.focus_browser_row_only(0);
    controller.set_browser_selected_paths(vec![PathBuf::from("clip.wav")]);
    controller.ui.browser.selection.autoscroll = false;
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("clip.wav"));
    controller.selection_state.range.set_range(Some(before_selection));
    controller.apply_selection(Some(before_selection));
    controller.set_edit_selection_range(before_edit);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.7,
    };
    controller.ui.waveform.cursor = Some(0.4);
    controller.ui.waveform.loop_enabled = false;
    let before = controller.capture_meaningful_ui_snapshot();

    controller.sample_view.wav.selected_wav = Some(PathBuf::from("clip_selection_001.wav"));
    controller.set_browser_selected_paths(vec![PathBuf::from("clip_selection_001.wav")]);
    controller.ui.browser.selection.autoscroll = true;
    controller.selection_state.range.set_range(None);
    controller.selection_state.edit_range.set_range(None);
    controller.apply_selection(None);
    controller.apply_edit_selection(None);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.0,
        end: 0.35,
    };
    controller.ui.waveform.cursor = Some(0.1);
    controller.ui.waveform.loop_enabled = true;
    let after = controller.capture_meaningful_ui_snapshot();

    let source_id = source.id.clone();
    let source_root = source.root.clone();
    let exported_relative = PathBuf::from("clip_selection_001.wav");
    let exported_absolute = source_root.join(&exported_relative);
    let backup_path = source_root.join("clip_selection_001.undo.wav");
    let entry = AppController::attach_meaningful_ui_restore(
        UndoEntry::new(
            "Deferred history restore",
            {
                let source_id = source_id.clone();
                let source_root = source_root.clone();
                let exported_relative = exported_relative.clone();
                let exported_absolute = exported_absolute.clone();
                move |_controller| {
                    Ok(UndoExecution::Deferred(UndoFileJob::RemoveSample {
                        source_id: source_id.clone(),
                        source_root: source_root.clone(),
                        relative_path: exported_relative.clone(),
                        absolute_path: exported_absolute.clone(),
                    }))
                }
            },
            move |_controller| {
                Ok(UndoExecution::Deferred(UndoFileJob::RestoreSample {
                    source_id: source_id.clone(),
                    source_root: source_root.clone(),
                    relative_path: exported_relative.clone(),
                    absolute_path: exported_absolute.clone(),
                    backup_path: backup_path.clone(),
                    tag: crate::sample_sources::Rating::KEEP_1,
                }))
            },
        ),
        before.clone(),
        after.clone(),
    );
    controller.push_undo_entry(entry);

    controller.undo();
    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Removed {
            source_id: source.id.clone(),
            relative_path: PathBuf::from("clip_selection_001.wav"),
        }),
        cancelled: false,
    }));

    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("clip.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(std::path::Path::new("clip.wav"))
    );
    assert!(!controller.ui.browser.selection.autoscroll);
    assert_eq!(controller.selection_state.range.range(), Some(before_selection));
    assert_eq!(controller.selection_state.edit_range.range(), Some(before_edit));
    assert_eq!(
        controller.ui.waveform.view,
        crate::app::state::WaveformView {
            start: 0.1,
            end: 0.7,
        }
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.4));
    assert!(!controller.ui.waveform.loop_enabled);
}
