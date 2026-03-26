use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use crate::app::controller::jobs::{FolderEntryMove, FolderMoveResult, FolderSampleMoveResult};
use crate::app::state::{DragPayload, DragSample, DragSource, DragTarget};
use crate::app_dirs::ConfigBaseGuard;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn sample_drop_to_folder_moves_and_updates_state() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(root.join("dest")).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&root.join("one.wav"), &[0.1, 0.2]);
    let metadata = std::fs::metadata(root.join("one.wav")).unwrap();
    let modified_ns = metadata
        .modified()
        .unwrap()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64;
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(Path::new("one.wav"), metadata.len(), modified_ns)
        .unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one.wav"),
    });
    controller.ui.drag.set_target(
        DragSource::Folders,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
    );
    controller.finish_active_drag();

    assert!(!root.join("one.wav").exists());
    assert!(root.join("dest").join("one.wav").is_file());
    assert!(
        controller
            .wav_index_for_path(Path::new("dest").join("one.wav").as_path())
            .is_some()
    );
}

#[test]
fn sample_drop_to_folder_rejects_conflicts() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let dest = root.join("dest");
    std::fs::create_dir_all(&dest).unwrap();
    write_test_wav(&root.join("one.wav"), &[0.1, 0.2]);
    write_test_wav(&dest.join("one.wav"), &[0.3, 0.4]);
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one.wav"),
    });
    controller.ui.drag.set_target(
        DragSource::Folders,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
    );
    controller.finish_active_drag();

    assert!(root.join("one.wav").is_file());
    assert!(dest.join("one.wav").is_file());
    assert!(
        controller
            .wav_index_for_path(Path::new("one.wav"))
            .is_some()
    );
}

#[test]
fn folder_drop_to_folder_moves_tree() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let src_folder = source.root.join("one");
    let dest_folder = source.root.join("dest");
    std::fs::create_dir_all(&src_folder).unwrap();
    std::fs::create_dir_all(&dest_folder).unwrap();
    write_test_wav(&src_folder.join("clip.wav"), &[0.1, 0.2]);

    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.ui.drag.payload = Some(DragPayload::Folder {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one"),
    });
    controller.ui.drag.set_target(
        DragSource::Folders,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
    );
    controller.finish_active_drag();

    assert!(!src_folder.exists());
    assert!(source.root.join("dest/one/clip.wav").is_file());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("dest/one/clip.wav")
    );
}

#[test]
fn sample_drop_to_folder_rejects_mixed_source_batches() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root_a = temp.path().join("source_a");
    let root_b = temp.path().join("source_b");
    std::fs::create_dir_all(root_a.join("dest")).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source_a = SampleSource::new(root_a);
    let source_b = SampleSource::new(root_b);
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_a.id.clone());

    controller.drag_drop().handle_samples_drop_to_folder(
        &[
            DragSample {
                source_id: source_a.id.clone(),
                relative_path: PathBuf::from("one.wav"),
            },
            DragSample {
                source_id: source_b.id.clone(),
                relative_path: PathBuf::from("two.wav"),
            },
        ],
        Path::new("dest"),
    );

    assert_eq!(
        controller.ui.status.text,
        "Samples must come from the same source to move into a folder"
    );
    assert!(controller.ui.progress.task.is_none());
}

#[test]
fn sample_drop_to_folder_rejects_selected_source_mismatch() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root_a = temp.path().join("source_a");
    let root_b = temp.path().join("source_b");
    std::fs::create_dir_all(root_a.join("dest")).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source_a = SampleSource::new(root_a);
    let source_b = SampleSource::new(root_b);
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_b.id.clone());

    controller.drag_drop().handle_samples_drop_to_folder(
        &[DragSample {
            source_id: source_a.id.clone(),
            relative_path: PathBuf::from("one.wav"),
        }],
        Path::new("dest"),
    );

    assert_eq!(
        controller.ui.status.text,
        "Switch to the sample's source before moving into its folders"
    );
    assert!(controller.ui.progress.task.is_none());
}

#[test]
fn folder_drop_to_folder_rejects_selected_source_mismatch() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root_a = temp.path().join("source_a");
    let root_b = temp.path().join("source_b");
    std::fs::create_dir_all(root_a.join("old")).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source_a = SampleSource::new(root_a);
    let source_b = SampleSource::new(root_b);
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_b.id.clone());

    controller.drag_drop().handle_folder_drop_to_folder(
        source_a.id.clone(),
        PathBuf::from("old"),
        Path::new("dest"),
    );

    assert_eq!(
        controller.ui.status.text,
        "Switch to the folder's source before moving it"
    );
    assert!(controller.ui.progress.task.is_none());
}

#[test]
fn folder_drop_to_folder_rejects_root_self_and_descendant_targets() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(root.join("old/sub")).unwrap();
    std::fs::create_dir_all(root.join("dest")).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root);
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::new(),
        Path::new("dest"),
    );
    assert_eq!(controller.ui.status.text, "Root folder cannot be moved");

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::from("old"),
        Path::new("old"),
    );
    assert_eq!(controller.ui.status.text, "Folder is already there");

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::from("old"),
        Path::new("old/sub"),
    );
    assert_eq!(
        controller.ui.status.text,
        "Cannot move a folder into itself"
    );
}

#[test]
fn apply_folder_sample_move_result_reports_cancelled_and_noop_statuses() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    controller
        .drag_drop()
        .apply_folder_sample_move_result(FolderSampleMoveResult {
            source_id: source.id.clone(),
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: true,
        });
    assert_eq!(controller.ui.status.text, "Move cancelled");

    controller
        .drag_drop()
        .apply_folder_sample_move_result(FolderSampleMoveResult {
            source_id: source.id,
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: false,
        });
    assert_eq!(controller.ui.status.text, "No samples moved");
}

#[test]
fn apply_folder_move_result_remaps_folder_state_and_focuses_destination() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(root.join("old/manual")).unwrap();
    std::fs::create_dir_all(root.join("dest/old/manual")).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    {
        let model = controller.current_folder_model_mut().unwrap();
        model.selected.insert(PathBuf::from("old"));
        model.expanded.insert(PathBuf::from("old"));
        model.focused = Some(PathBuf::from("old"));
        model.selection_anchor = Some(PathBuf::from("old"));
        model.manual_folders.insert(PathBuf::from("old/manual"));
    }
    controller.ui.sources.folders.last_focused_path = Some(PathBuf::from("old"));

    controller
        .drag_drop()
        .apply_folder_move_result(FolderMoveResult {
            source_id: source.id,
            old_folder: PathBuf::from("old"),
            new_folder: PathBuf::from("dest/old"),
            folder_moved: true,
            moved: vec![FolderEntryMove {
                old_relative: PathBuf::from("old/clip.wav"),
                new_relative: PathBuf::from("dest/old/clip.wav"),
                file_size: 0,
                modified_ns: 0,
                tag: crate::sample_sources::Rating::NEUTRAL,
                looped: false,
                locked: false,
                last_played_at: None,
            }],
            errors: Vec::new(),
            cancelled: false,
        });

    let model = controller.current_folder_model().unwrap();
    assert!(model.selected.contains(Path::new("dest/old")));
    assert!(model.expanded.contains(Path::new("dest/old")));
    assert_eq!(model.focused.as_deref(), Some(Path::new("dest/old")));
    assert_eq!(
        model.selection_anchor.as_deref(),
        Some(Path::new("dest/old"))
    );
    assert!(model.manual_folders.contains(Path::new("dest/old/manual")));
    assert_eq!(
        controller.ui.sources.folders.last_focused_path.as_deref(),
        Some(Path::new("dest/old"))
    );
    assert_eq!(controller.ui.status.text, "Moved folder to dest/old");
}
