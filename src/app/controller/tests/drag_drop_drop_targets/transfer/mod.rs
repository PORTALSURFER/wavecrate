use super::super::super::test_support::write_test_wav;
use super::super::super::*;
use super::Must;
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferResult, DropTargetTransferSuccess,
};
use crate::app::state::{DragPayload, DragSample, DragSource, DragTarget};
use crate::sample_sources::db::DB_FILE_NAME;
use crate::sample_sources::{Rating, SampleSource, SourceId, WavEntry};
use std::path::{Path, PathBuf};

mod apply_result;
mod workflow;

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
