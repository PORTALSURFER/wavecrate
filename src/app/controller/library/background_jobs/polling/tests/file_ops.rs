use super::*;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage,
    FileOpResult, RetainedDeleteResolutionMode, RetainedDeleteResolutionResult,
};
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::ProgressTaskKind;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool, mpsc::channel};

#[test]
fn file_ops_messages_update_progress_and_clear_active_overlay_on_finish() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(ProgressTaskKind::FileOps, "Copying files", 5, true);
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 2,
        detail: Some("Copying kick.wav".into()),
    }));

    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Copying kick.wav")
    );
    assert!(controller.runtime.jobs.file_ops_in_progress());

    let result = FileOpResult::ClipboardPaste(ClipboardPasteResult {
        outcome: ClipboardPasteOutcome::Source {
            source_id: source.id,
            added: Vec::new(),
        },
        skipped: 0,
        errors: Vec::new(),
        cancelled: true,
        target_label: "Source".into(),
        action_past_tense: "Pasted",
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
    assert_eq!(controller.ui.progress.task, None);
}

#[test]
fn retained_delete_resolution_result_clears_busy_scope_and_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(
        ProgressTaskKind::FileOps,
        "Restoring retained deletes",
        1,
        false,
    );
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: Vec::new(),
    });
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    let result = FileOpResult::RetainedDeleteResolution(RetainedDeleteResolutionResult {
        mode: RetainedDeleteResolutionMode::Restore,
        resolved: 1,
        affected_sources: vec![source.id],
        scan_sources: Vec::new(),
        failures: Vec::new(),
        recovery_report:
            crate::app::controller::library::source_folders::delete_recovery::DeleteRecoveryReport {
                entries: Vec::new(),
                retained_entries: Vec::new(),
                errors: Vec::new(),
            },
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    assert!(
        controller
            .runtime
            .active_retained_delete_resolution
            .is_none()
    );
    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
}

#[test]
fn selection_export_progress_message_updates_status_bar_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.jobs.set_pending_slice_batch_export(Some(
        crate::app::controller::jobs::PendingSliceBatchExport {
            request_id: 23,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("clip.wav"),
        },
    ));

    controller.handle_background_job_message(JobMessage::SelectionExport(
        SelectionExportMessage::Progress {
            request_id: 23,
            total: 4,
            completed: 2,
            detail: Some("Saving clip_slice002.wav".into()),
        },
    ));

    assert!(controller.ui.progress.visible);
    assert!(!controller.ui.progress.modal);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::SelectionExport)
    );
    assert_eq!(controller.ui.progress.total, 4);
    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Saving clip_slice002.wav")
    );
}
