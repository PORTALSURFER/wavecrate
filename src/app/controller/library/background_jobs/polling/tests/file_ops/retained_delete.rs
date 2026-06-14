use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, FileOpMessage, FileOpResult, JobMessage,
    RetainedDeleteResolutionMode, RetainedDeleteResolutionResult,
};
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::ProgressTaskKind;
use std::sync::{Arc, atomic::AtomicBool, mpsc::channel};

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
    controller
        .runtime
        .recovery
        .active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
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
                scan_sources: Vec::new(),
                errors: Vec::new(),
            },
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    assert!(
        controller
            .runtime
            .recovery
            .active_retained_delete_resolution
            .is_none()
    );
    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
}
