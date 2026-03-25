use super::super::test_support::write_test_wav;
use super::super::*;
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferResult, DropTargetTransferSuccess,
};
use crate::app::state::{DragPayload, DragSample, DragSource, DragTarget};
use crate::app_dirs::ConfigBaseGuard;
use crate::app_core::state::StatusTone;
use crate::sample_sources::config::DropTargetConfig;
use crate::sample_sources::db::DB_FILE_NAME;
use crate::sample_sources::{Rating, SampleSource, SourceId, WavEntry};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;

trait Must<T> {
    fn must(self) -> T;
}

impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
    fn must(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("unexpected error: {err}"),
        }
    }
}

impl<T> Must<T> for Option<T> {
    fn must(self) -> T {
        match self {
            Some(value) => value,
            None => panic!("expected value, found none"),
        }
    }
}

fn setup_cross_source_drop_fixture(
    temp: &tempfile::TempDir,
) -> (AppController, SampleSource, SampleSource, PathBuf) {
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    let target_drop = target_root.join("dest");
    std::fs::create_dir_all(&source_root).must();
    std::fs::create_dir_all(&target_drop).must();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root);
    let target = SampleSource::new(target_root);
    controller.library.sources.push(source.clone());
    controller.library.sources.push(target.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).must();
    controller.cache_db(&target).must();
    (controller, source, target, target_drop)
}

fn sample_modified_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .must()
        .modified()
        .must()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .must()
        .as_nanos() as i64
}

fn seed_source_sample(controller: &mut AppController, source: &SampleSource, relative: &str) {
    let absolute = source.root.join(relative);
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent).must();
    }
    write_test_wav(&absolute, &[0.1, 0.2, -0.2, 0.3]);
    let metadata = std::fs::metadata(&absolute).must();
    let modified_ns = sample_modified_ns(&absolute);
    let db = controller.database_for(source).must();
    db.upsert_file(Path::new(relative), metadata.len(), modified_ns)
        .must();
    db.set_tag(Path::new(relative), Rating::KEEP_1).must();
    db.set_looped(Path::new(relative), true).must();
    db.set_last_played_at(Path::new(relative), 42).must();
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: PathBuf::from(relative),
        file_size: metadata.len(),
        modified_ns,
        content_hash: None,
        tag: Rating::KEEP_1,
        looped: true,
        locked: true,
        missing: false,
        last_played_at: Some(42),
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
}

fn set_source_samples_for_tests(
    controller: &mut AppController,
    source: &SampleSource,
    relatives: &[&str],
) {
    let entries = relatives
        .iter()
        .map(|relative| {
            let absolute = source.root.join(relative);
            let metadata = std::fs::metadata(&absolute).must();
            WavEntry {
                relative_path: PathBuf::from(relative),
                file_size: metadata.len(),
                modified_ns: sample_modified_ns(&absolute),
                content_hash: None,
                tag: Rating::KEEP_1,
                looped: true,
                locked: true,
                missing: false,
                last_played_at: Some(42),
            }
        })
        .collect();
    controller.set_wav_entries_for_tests(entries);
    let db = controller.database_for(source).must();
    for relative in relatives {
        db.set_looped(Path::new(relative), true).must();
        db.set_locked(Path::new(relative), true).must();
        db.set_last_played_at(Path::new(relative), 42).must();
    }
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
}

fn seed_target_collision(
    controller: &mut AppController,
    target: &SampleSource,
    relative: &str,
    tag: Rating,
) {
    let absolute = target.root.join(relative);
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent).must();
    }
    write_test_wav(&absolute, &[0.0, 0.1]);
    let metadata = std::fs::metadata(&absolute).must();
    let modified_ns = sample_modified_ns(&absolute);
    let db = controller.database_for(target).must();
    db.upsert_file(Path::new(relative), metadata.len(), modified_ns)
        .must();
    db.set_tag(Path::new(relative), tag).must();
}

fn finish_drop(
    controller: &mut AppController,
    source_id: SourceId,
    relative_path: &str,
    target_path: &Path,
    copy_requested: bool,
) {
    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id,
        relative_path: PathBuf::from(relative_path),
    });
    controller.ui.drag.copy_on_drop = copy_requested;
    controller.ui.drag.set_target(
        DragSource::DropTargets,
        DragTarget::DropTarget {
            path: target_path.to_path_buf(),
        },
    );
    controller.finish_active_drag();
}

fn finish_multi_sample_drop(
    controller: &mut AppController,
    samples: Vec<DragSample>,
    target_path: &Path,
    copy_requested: bool,
) {
    controller.ui.drag.payload = Some(DragPayload::Samples { samples });
    controller.ui.drag.copy_on_drop = copy_requested;
    controller.ui.drag.set_target(
        DragSource::DropTargets,
        DragTarget::DropTarget {
            path: target_path.to_path_buf(),
        },
    );
    controller.finish_active_drag();
}

fn db_entry(
    controller: &mut AppController,
    source: &SampleSource,
    relative: &str,
) -> Option<WavEntry> {
    controller
        .database_for(source)
        .must()
        .list_files()
        .must()
        .into_iter()
        .find(|entry| entry.relative_path == PathBuf::from(relative))
}

fn drop_target_transfer_result(
    kind: DropTargetTransferKind,
    target: &SampleSource,
    transferred: Vec<DropTargetTransferSuccess>,
    errors: Vec<&str>,
    cancelled: bool,
) -> DropTargetTransferResult {
    DropTargetTransferResult {
        kind,
        target_source_id: target.id.clone(),
        target_label: String::from("dest"),
        transferred,
        errors: errors.into_iter().map(String::from).collect(),
        cancelled,
    }
}

fn transferred_sample(
    source: &SampleSource,
    source_relative: &str,
    target_relative: &str,
) -> DropTargetTransferSuccess {
    DropTargetTransferSuccess {
        source_id: source.id.clone(),
        source_relative: PathBuf::from(source_relative),
        target_relative: PathBuf::from(target_relative),
        file_size: 16,
        modified_ns: 123,
        tag: Rating::KEEP_1,
        looped: true,
        locked: true,
        last_played_at: Some(42),
    }
}

fn lock_db_until_released(
    source_root: &Path,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).must();
        conn.execute_batch("BEGIN IMMEDIATE").must();
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().must();
    (lock_release_tx, lock_done_rx)
}

#[test]
fn drop_target_copy_duplicates_sample() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let dest = root.join("dest");
    std::fs::create_dir_all(&dest).unwrap();
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
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one.wav"),
    });
    controller.ui.drag.copy_on_drop = true;
    controller.ui.drag.set_target(
        DragSource::DropTargets,
        DragTarget::DropTarget { path: dest.clone() },
    );
    controller.finish_active_drag();

    assert!(root.join("one.wav").is_file());
    assert!(dest.join("one.wav").is_file());

    let entries = db.list_files().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.relative_path == PathBuf::from("one.wav"))
    );
    assert!(entries.iter().any(|entry| {
        entry.relative_path == PathBuf::from("dest/one.wav") && entry.tag == Rating::KEEP_1
    }));
}

#[test]
fn cross_source_drop_target_move_preserves_metadata() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");

    finish_drop(
        &mut controller,
        source.id.clone(),
        "one.wav",
        &target_drop,
        false,
    );

    assert!(!source.root.join("one.wav").exists());
    assert!(target_drop.join("one.wav").is_file());
    assert!(db_entry(&mut controller, &source, "one.wav").is_none());
    let moved = db_entry(&mut controller, &target, "dest/one.wav").must();
    assert_eq!(moved.tag, Rating::KEEP_1);
    assert!(moved.looped);
    assert!(moved.locked);
    assert_eq!(moved.last_played_at, Some(42));
}

#[test]
fn cross_source_drop_target_copy_uses_collision_suffix() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");
    seed_target_collision(&mut controller, &target, "dest/one.wav", Rating::TRASH_1);

    finish_drop(
        &mut controller,
        source.id.clone(),
        "one.wav",
        &target_drop,
        true,
    );

    assert!(source.root.join("one.wav").is_file());
    assert!(target_drop.join("one.wav").is_file());
    assert!(target_drop.join("one_copy001.wav").is_file());
    let copied = db_entry(&mut controller, &target, "dest/one_copy001.wav").must();
    assert_eq!(copied.tag, Rating::KEEP_1);
    assert!(copied.looped);
    assert!(copied.locked);
    assert_eq!(copied.last_played_at, Some(42));
}

#[test]
fn cross_source_drop_target_multi_copy_batches_samples() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");
    seed_source_sample(&mut controller, &source, "two.wav");
    set_source_samples_for_tests(&mut controller, &source, &["one.wav", "two.wav"]);

    finish_multi_sample_drop(
        &mut controller,
        vec![
            DragSample {
                source_id: source.id.clone(),
                relative_path: PathBuf::from("one.wav"),
            },
            DragSample {
                source_id: source.id.clone(),
                relative_path: PathBuf::from("two.wav"),
            },
        ],
        &target_drop,
        true,
    );

    assert!(source.root.join("one.wav").is_file());
    assert!(source.root.join("two.wav").is_file());
    assert!(target_drop.join("one.wav").is_file());
    assert!(target_drop.join("two.wav").is_file());
    let copied_one = db_entry(&mut controller, &target, "dest/one.wav").must();
    let copied_two = db_entry(&mut controller, &target, "dest/two.wav").must();
    assert_eq!(copied_one.tag, Rating::KEEP_1);
    assert!(copied_one.looped);
    assert!(copied_one.locked);
    assert_eq!(copied_one.last_played_at, Some(42));
    assert_eq!(copied_two.tag, Rating::KEEP_1);
    assert!(copied_two.looped);
    assert!(copied_two.locked);
    assert_eq!(copied_two.last_played_at, Some(42));
}

#[test]
fn cross_source_drop_target_copy_rolls_back_when_target_db_is_locked() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&target.root);

    finish_drop(
        &mut controller,
        source.id.clone(),
        "one.wav",
        &target_drop,
        true,
    );
    let _ = lock_release_tx.send(());
    lock_done_rx.recv_timeout(Duration::from_secs(1)).must();

    assert!(source.root.join("one.wav").is_file());
    assert!(!target_drop.join("one.wav").exists());
    assert!(db_entry(&mut controller, &source, "one.wav").is_some());
    assert!(db_entry(&mut controller, &target, "dest/one.wav").is_none());
}

#[test]
fn cross_source_drop_target_move_removes_target_row_when_source_cleanup_fails() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    finish_drop(
        &mut controller,
        source.id.clone(),
        "one.wav",
        &target_drop,
        false,
    );
    let _ = lock_release_tx.send(());
    lock_done_rx.recv_timeout(Duration::from_secs(1)).must();

    assert!(source.root.join("one.wav").is_file());
    assert!(!target_drop.join("one.wav").exists());
    assert!(db_entry(&mut controller, &source, "one.wav").is_some());
    assert!(db_entry(&mut controller, &target, "dest/one.wav").is_none());
}

#[test]
fn cross_source_drop_target_missing_source_is_rejected() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, _target, target_drop) = setup_cross_source_drop_fixture(&temp);
    seed_source_sample(&mut controller, &source, "one.wav");

    finish_drop(
        &mut controller,
        SourceId::from_string("missing"),
        "one.wav",
        &target_drop,
        false,
    );

    assert!(source.root.join("one.wav").is_file());
    assert!(!target_drop.join("one.wav").exists());
    assert!(db_entry(&mut controller, &source, "one.wav").is_some());
}

#[test]
fn cross_source_drop_target_outside_configured_sources_is_rejected() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, _target, _target_drop) = setup_cross_source_drop_fixture(&temp);
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(&outside).must();
    seed_source_sample(&mut controller, &source, "one.wav");

    finish_drop(
        &mut controller,
        source.id.clone(),
        "one.wav",
        &outside,
        false,
    );

    assert!(source.root.join("one.wav").is_file());
    assert!(!outside.join("one.wav").exists());
    assert!(db_entry(&mut controller, &source, "one.wav").is_some());
}

#[test]
fn apply_drop_target_transfer_result_reports_cancelled_statuses() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, _source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            Vec::new(),
            Vec::new(),
            true,
        ));
    assert_eq!(controller.ui.status.text, "Copy cancelled");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Move,
            &target,
            Vec::new(),
            Vec::new(),
            true,
        ));
    assert_eq!(controller.ui.status.text, "Move cancelled");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_drop_target_transfer_result_reports_noop_statuses() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, _source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            Vec::new(),
            Vec::new(),
            false,
        ));
    assert_eq!(controller.ui.status.text, "No samples copied");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Move,
            &target,
            Vec::new(),
            Vec::new(),
            false,
        ));
    assert_eq!(controller.ui.status.text, "No samples moved");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_drop_target_transfer_result_reports_partial_errors_with_warning_tone() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            vec![transferred_sample(&source, "one.wav", "dest/one.wav")],
            vec!["target already contains clip"],
            false,
        ));

    assert_eq!(
        controller.ui.status.text,
        "Copied 1 sample(s) to dest with 1 error(s)"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn drop_target_panel_accepts_folder_drag() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let target = root.join("targets");
    std::fs::create_dir_all(&target).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());

    controller.ui.drag.payload = Some(DragPayload::Folder {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("targets"),
    });
    controller
        .ui
        .drag
        .set_target(DragSource::DropTargets, DragTarget::DropTargetsPanel);
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets.len(), 1);
    assert_eq!(
        controller.settings.drop_targets[0].path,
        root.join("targets")
    );
}

#[test]
fn drop_target_drag_reorders_list() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let a = root.join("a");
    let b = root.join("b");
    let c = root.join("c");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::create_dir_all(&c).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source);
    controller.settings.drop_targets = vec![
        DropTargetConfig::new(a.clone()),
        DropTargetConfig::new(b.clone()),
        DropTargetConfig::new(c.clone()),
    ];
    controller.refresh_drop_targets_ui();

    controller.ui.drag.payload = Some(DragPayload::DropTargetReorder { path: a.clone() });
    controller.ui.drag.set_target(
        DragSource::DropTargets,
        DragTarget::DropTarget { path: c.clone() },
    );
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets[0].path, b);
    assert_eq!(controller.settings.drop_targets[1].path, a);
    assert_eq!(controller.settings.drop_targets[2].path, c);

    controller.ui.drag.payload = Some(DragPayload::DropTargetReorder { path: a.clone() });
    controller
        .ui
        .drag
        .set_target(DragSource::DropTargets, DragTarget::DropTargetsPanel);
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets[0].path, b);
    assert_eq!(controller.settings.drop_targets[1].path, c);
    assert_eq!(controller.settings.drop_targets[2].path, a);
}
