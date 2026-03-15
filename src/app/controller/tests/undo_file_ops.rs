use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use super::super::*;
use crate::app::controller::jobs::{FileOpResult, UndoFileJob, UndoFileOpResult, UndoFileOutcome};
use crate::app::controller::undo::{DeferredUndo, UndoDirection, UndoEntry, UndoExecution};
use std::path::{Path, PathBuf};

fn deferred_test_entry(
    label: &str,
    undo_value: bool,
    redo_value: bool,
) -> UndoEntry<AppController> {
    let label = label.to_string();
    UndoEntry::new(
        label,
        move |controller: &mut AppController| {
            controller.settings.controls.advance_after_rating = undo_value;
            Ok(UndoExecution::Applied)
        },
        move |controller: &mut AppController| {
            controller.settings.controls.advance_after_rating = redo_value;
            Ok(UndoExecution::Applied)
        },
    )
}

fn deferred_remove_job(source: &SampleSource, relative_path: &str) -> UndoFileJob {
    let relative_path = PathBuf::from(relative_path);
    UndoFileJob::RemoveSample {
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        absolute_path: source.root.join(&relative_path),
        relative_path,
    }
}

#[test]
fn deferred_undo_success_updates_entry_and_pushes_redo() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.settings.controls.advance_after_rating = false;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("remove sample", false, true),
        direction: UndoDirection::Undo,
        job: UndoFileJob::Overwrite {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: PathBuf::from("one.wav"),
            absolute_path: source.root.join("one.wav"),
            backup_path: source.root.join("undo-before.wav"),
        },
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Overwrite {
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            file_size: 42,
            modified_ns: 7,
            tag: crate::sample_sources::Rating::KEEP_1,
            looped: true,
            last_played_at: Some(11),
        }),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());
    let updated_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    let updated = controller.wav_entry(updated_index).unwrap();
    assert_eq!(updated.file_size, 42);
    assert_eq!(updated.modified_ns, 7);
    assert_eq!(updated.tag, crate::sample_sources::Rating::KEEP_1);
    assert!(updated.looped);
    assert_eq!(updated.last_played_at, Some(11));

    controller.redo();
    assert!(controller.settings.controls.advance_after_rating);
}

#[test]
fn deferred_undo_cancellation_restores_undo_entry() {
    let (mut controller, source) = dummy_controller();
    controller.settings.controls.advance_after_rating = true;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("deferred undo", false, true),
        direction: UndoDirection::Undo,
        job: deferred_remove_job(&source, "one.wav"),
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Err("ignored after cancellation".to_string()),
        cancelled: true,
    }));

    assert!(controller.history.pending_undo.is_none());

    controller.undo();
    assert!(!controller.settings.controls.advance_after_rating);
}

#[test]
fn deferred_redo_failure_restores_redo_entry() {
    let (mut controller, source) = dummy_controller();
    controller.settings.controls.advance_after_rating = false;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("deferred redo", false, true),
        direction: UndoDirection::Redo,
        job: deferred_remove_job(&source, "one.wav"),
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Err("redo failed".to_string()),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());

    controller.redo();
    assert!(controller.settings.controls.advance_after_rating);
}
