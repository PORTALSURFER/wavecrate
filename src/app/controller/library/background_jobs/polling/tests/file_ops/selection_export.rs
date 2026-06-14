use crate::app::controller::jobs::{JobMessage, SelectionExportMessage};
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::ProgressTaskKind;
use std::path::PathBuf;

#[test]
fn selection_export_file_op_progress_message_updates_status_bar_progress() {
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
