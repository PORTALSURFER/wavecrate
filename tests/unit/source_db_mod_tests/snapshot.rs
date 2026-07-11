use super::*;
use crate::sample_sources::database_path_for;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn snapshot_includes_committed_wal_resident_metadata() {
    let source_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let destination = database_path_for(destination_root.path());
    let source = SourceDatabase::open(source_root.path()).unwrap();
    source
        .connection
        .pragma_update(None, "wal_autocheckpoint", 0)
        .unwrap();
    source.upsert_file(Path::new("wal.wav"), 10, 5).unwrap();
    source
        .set_tag(Path::new("wal.wav"), Rating::KEEP_1)
        .unwrap();
    let wal = snapshot_sidecar_path(&database_path_for(source_root.path()), "-wal");
    assert!(wal.metadata().unwrap().len() > 0);

    source.snapshot_to_path(&destination).unwrap();

    let snapshot = SourceDatabase::open(destination_root.path()).unwrap();
    let entry = snapshot
        .entry_for_path(Path::new("wal.wav"))
        .unwrap()
        .unwrap();
    assert_eq!(entry.tag, Rating::KEEP_1);
}

#[test]
fn snapshot_waits_for_source_writer_and_captures_committed_state() {
    let source_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let destination = database_path_for(destination_root.path());
    let source = SourceDatabase::open(source_root.path()).unwrap();
    source.upsert_file(Path::new("writer.wav"), 10, 5).unwrap();
    let mut writer = source.write_batch().unwrap();
    writer
        .set_tag(Path::new("writer.wav"), Rating::KEEP_3)
        .unwrap();
    let source_path = source_root.path().to_path_buf();
    let (done_tx, done_rx) = mpsc::channel();

    let snapshot_thread = std::thread::spawn(move || {
        let source = SourceDatabase::open(&source_path).unwrap();
        let result = source.snapshot_to_path(&destination);
        done_tx.send(result).unwrap();
    });
    assert!(done_rx.recv_timeout(Duration::from_millis(50)).is_err());
    writer.commit().unwrap();
    done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("snapshot completion")
        .unwrap();
    snapshot_thread.join().unwrap();

    let snapshot = SourceDatabase::open(destination_root.path()).unwrap();
    let entry = snapshot
        .entry_for_path(Path::new("writer.wav"))
        .unwrap()
        .unwrap();
    assert_eq!(entry.tag, Rating::KEEP_3);
}

#[test]
fn retained_snapshot_fence_blocks_later_source_writers_until_publish() {
    let source_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let destination = database_path_for(destination_root.path());
    let source = SourceDatabase::open(source_root.path()).unwrap();
    let fence = source
        .snapshot_to_path_with_write_fence(&destination)
        .unwrap();
    let source_path = source_root.path().to_path_buf();
    let (done_tx, done_rx) = mpsc::channel();

    let writer_thread = std::thread::spawn(move || {
        let source = SourceDatabase::open(source_path).unwrap();
        done_tx
            .send(source.set_metadata("after_snapshot", "committed"))
            .unwrap();
    });
    assert!(done_rx.recv_timeout(Duration::from_millis(50)).is_err());

    drop(fence);
    done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("writer completion")
        .unwrap();
    writer_thread.join().unwrap();
}

#[test]
fn snapshot_does_not_recreate_a_missing_destination_root() {
    let source_root = tempdir().unwrap();
    let destination_parent = tempdir().unwrap();
    let missing_root = destination_parent.path().join("removed");
    let destination = database_path_for(&missing_root);
    let source = SourceDatabase::open(source_root.path()).unwrap();

    source.snapshot_to_path(&destination).unwrap_err();

    assert!(!missing_root.exists());
}

#[test]
fn snapshot_failure_does_not_remove_preexisting_destination() {
    let source_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let destination = database_path_for(destination_root.path());
    std::fs::write(&destination, b"preexisting").unwrap();
    let source = SourceDatabase::open(source_root.path()).unwrap();

    source.snapshot_to_path(&destination).unwrap_err();

    assert_eq!(std::fs::read(destination).unwrap(), b"preexisting");
}

#[cfg(unix)]
#[test]
fn snapshot_rejects_broken_symlink_destination_without_following_or_removing_it() {
    use std::os::unix::fs::symlink;

    let source_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let destination = database_path_for(destination_root.path());
    let outside_target = destination_root.path().join("outside-target.db");
    symlink(&outside_target, &destination).unwrap();
    let source = SourceDatabase::open(source_root.path()).unwrap();

    source.snapshot_to_path(&destination).unwrap_err();

    assert!(std::fs::symlink_metadata(&destination).is_ok());
    assert!(!outside_target.exists());
}

#[cfg(unix)]
#[test]
fn snapshot_revalidates_source_path_before_reopening() {
    use std::os::unix::fs::symlink;

    let source_root = tempdir().unwrap();
    let outside_root = tempdir().unwrap();
    let destination_root = tempdir().unwrap();
    let source = SourceDatabase::open(source_root.path()).unwrap();
    source.upsert_file(Path::new("trusted.wav"), 1, 1).unwrap();
    let outside = SourceDatabase::open(outside_root.path()).unwrap();
    outside.upsert_file(Path::new("outside.wav"), 2, 2).unwrap();
    let source_path = database_path_for(source_root.path());
    std::fs::remove_file(&source_path).unwrap();
    symlink(database_path_for(outside_root.path()), &source_path).unwrap();
    let destination = database_path_for(destination_root.path());

    let error = source.snapshot_to_path(&destination).unwrap_err();

    assert!(matches!(
        error,
        SourceDbError::UnsafeSourceDatabasePath { .. }
    ));
    assert!(!destination.exists());
}
