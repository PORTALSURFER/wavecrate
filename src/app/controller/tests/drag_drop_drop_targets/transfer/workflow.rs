use super::*;
use crate::app_dirs::ConfigBaseGuard;
use std::time::Duration;
use tempfile::tempdir;

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
