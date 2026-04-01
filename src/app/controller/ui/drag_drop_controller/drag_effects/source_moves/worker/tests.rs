use super::super::source_move_test_guard;
use super::*;
use crate::app::controller::jobs::SourceMoveRequest;
use crate::app::controller::test_support::write_test_wav;
use crate::sample_sources::{SampleSource, SourceDatabase};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn source_move_task_uses_unique_target_name_on_collision() {
    let _guard = source_move_test_guard();
    set_before_source_move_target_db_stage_hook(None);
    set_after_source_move_progress_hook(None);
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&target_root).unwrap();
    let source = SampleSource::new(source_root.clone());
    write_test_wav(&source_root.join("one.wav"), &[0.0, 0.1, -0.1]);
    write_test_wav(&target_root.join("one.wav"), &[0.0, 0.2, -0.2]);
    let source_db = SourceDatabase::open(&source_root).unwrap();
    source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
    source_db
        .set_tag(Path::new("one.wav"), crate::sample_sources::Rating::KEEP_1)
        .unwrap();
    source_db.set_looped(Path::new("one.wav"), true).unwrap();
    source_db.set_locked(Path::new("one.wav"), true).unwrap();
    source_db
        .set_last_played_at(Path::new("one.wav"), 42)
        .unwrap();
    let cancel = Arc::new(AtomicBool::new(false));

    let result = run_source_move_task(
        SourceId::from_string("target"),
        target_root.clone(),
        vec![SourceMoveRequest {
            source_id: source.id,
            source_root: source_root.clone(),
            relative_path: PathBuf::from("one.wav"),
        }],
        Vec::new(),
        cancel,
        None,
    );

    assert_eq!(result.moved.len(), 1);
    assert_eq!(
        result.moved[0].target_relative,
        PathBuf::from("one_move001.wav")
    );
    assert!(result.moved[0].looped);
    assert!(result.moved[0].locked);
    assert_eq!(result.moved[0].last_played_at, Some(42));
    assert!(target_root.join("one_move001.wav").is_file());
    assert!(!source_root.join("one.wav").exists());
    let target_db = SourceDatabase::open(&target_root).unwrap();
    assert_eq!(
        target_db
            .tag_for_path(Path::new("one_move001.wav"))
            .unwrap(),
        Some(crate::sample_sources::Rating::KEEP_1)
    );
    assert_eq!(
        target_db
            .looped_for_path(Path::new("one_move001.wav"))
            .unwrap(),
        Some(true)
    );
    assert_eq!(
        target_db
            .locked_for_path(Path::new("one_move001.wav"))
            .unwrap(),
        Some(true)
    );
}

#[test]
fn source_move_task_rolls_back_when_target_db_stage_fails() {
    let _guard = source_move_test_guard();
    set_before_source_move_target_db_stage_hook(None);
    set_after_source_move_progress_hook(None);
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&target_root).unwrap();
    let source = SampleSource::new(source_root.clone());
    write_test_wav(&source_root.join("one.wav"), &[0.0, 0.1, -0.1]);
    let source_db = SourceDatabase::open(&source_root).unwrap();
    source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
    set_before_source_move_target_db_stage_hook(Some(Box::new(|| {
        Err("Simulated target DB failure".into())
    })));

    let result = run_source_move_task(
        SourceId::from_string("target"),
        target_root.clone(),
        vec![SourceMoveRequest {
            source_id: source.id,
            source_root: source_root.clone(),
            relative_path: PathBuf::from("one.wav"),
        }],
        Vec::new(),
        Arc::new(AtomicBool::new(false)),
        None,
    );

    assert!(source_root.join("one.wav").is_file());
    assert!(result.moved.is_empty());
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.contains("Simulated target DB failure"))
    );
}

#[test]
fn source_move_task_reports_progress_once_for_missing_file_request() {
    let _guard = source_move_test_guard();
    set_before_source_move_target_db_stage_hook(None);
    set_after_source_move_progress_hook(None);
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&target_root).unwrap();
    let request = SourceMoveRequest {
        source_id: SourceId::from_string("source"),
        source_root,
        relative_path: PathBuf::from("missing.wav"),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();

    let result = run_source_move_task(
        SourceId::from_string("target"),
        target_root,
        vec![request],
        Vec::new(),
        cancel,
        Some(&tx),
    );

    assert!(result.moved.is_empty());
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("File missing"));
    let progress_messages = rx
        .try_iter()
        .filter(|message| matches!(message, FileOpMessage::Progress { .. }))
        .count();
    assert_eq!(progress_messages, 1);
}

#[test]
fn source_move_task_cancels_after_first_completed_request() {
    let _guard = source_move_test_guard();
    set_before_source_move_target_db_stage_hook(None);
    set_after_source_move_progress_hook(None);
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&target_root).unwrap();
    let source = SampleSource::new(source_root.clone());
    for name in ["one.wav", "two.wav"] {
        write_test_wav(&source_root.join(name), &[0.0, 0.1, -0.1]);
    }
    let source_db = SourceDatabase::open(&source_root).unwrap();
    source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
    source_db.upsert_file(Path::new("two.wav"), 3, 1).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_hook = cancel.clone();
    set_after_source_move_progress_hook(Some(Box::new(move |completed| {
        if completed == 1 {
            cancel_for_hook.store(true, Ordering::Relaxed);
        }
    })));

    let result = run_source_move_task(
        SourceId::from_string("target"),
        target_root.clone(),
        vec![
            SourceMoveRequest {
                source_id: source.id.clone(),
                source_root: source_root.clone(),
                relative_path: PathBuf::from("one.wav"),
            },
            SourceMoveRequest {
                source_id: source.id,
                source_root: source_root.clone(),
                relative_path: PathBuf::from("two.wav"),
            },
        ],
        Vec::new(),
        cancel,
        None,
    );

    assert!(result.cancelled);
    assert_eq!(result.moved.len(), 1);
    assert!(target_root.join(&result.moved[0].target_relative).is_file());
    assert!(source_root.join("two.wav").is_file());
    set_after_source_move_progress_hook(None);
}
