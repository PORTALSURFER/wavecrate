use crate::app::controller::jobs::{
    ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage, FileOpResult, JobMessage,
};
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::ProgressTaskKind;
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
        item: None,
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
