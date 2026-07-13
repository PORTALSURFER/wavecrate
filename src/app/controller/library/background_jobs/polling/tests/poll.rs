use super::*;
use crate::app::controller::jobs::{
    FileOpMessage, FileOpResult, SampleAutoRenameResult, StarmapWriteOutcome, UmapBuildJob,
    UmapBuildResult,
};
use crate::app::controller::test_support::dummy_controller;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn poll_background_jobs_limits_messages_per_pass() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let sender = controller.runtime.jobs.message_sender();
    for _ in 0..(MAX_BACKGROUND_MESSAGES_PER_POLL + 2) {
        sender
            .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                source_id: Some(source.id.clone()),
                progress: crate::app::controller::library::analysis_jobs::AnalysisProgress {
                    pending: 2,
                    running: 1,
                    done: 3,
                    failed: 0,
                    samples_total: 5,
                    samples_pending_or_running: 2,
                },
            }))
            .expect("queue analysis progress");
    }

    controller.poll_background_jobs();

    let mut remaining = 0usize;
    while controller.runtime.jobs.try_recv_message().is_ok() {
        remaining += 1;
    }
    assert_eq!(remaining, 2);
}

#[test]
fn poll_background_jobs_resumes_starmap_write_after_file_op_finishes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let pending_path = PathBuf::from("sample.wav");
    controller.begin_pending_file_mutation(&source.id, [pending_path.clone()]);
    controller.apply_background_job_message_for_tests(JobMessage::UmapBuilt(UmapBuildResult {
        job: UmapBuildJob {
            model_id: "model-v1".to_string(),
            umap_version: "umap-v1".to_string(),
            source_id: source.id.clone(),
        },
        result: Ok(StarmapWriteOutcome::DeferredForFileOp),
    }));
    controller
        .runtime
        .jobs
        .message_sender()
        .send(JobMessage::FileOps(FileOpMessage::Finished(
            FileOpResult::SampleAutoRename(SampleAutoRenameResult {
                source_id: source.id.clone(),
                requested_paths: vec![pending_path],
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: Vec::new(),
            }),
        )))
        .expect("queue file-op completion");

    controller.poll_background_jobs();

    assert!(!controller.source_has_pending_file_mutations(&source.id));
    assert!(controller.runtime.jobs.umap_build_in_progress());
    assert!(
        controller
            .runtime
            .jobs
            .take_ready_deferred_umap_build_for_tests()
            .is_none(),
        "the post-drain retry should already have promoted the deferred layout job"
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while controller.runtime.jobs.umap_build_in_progress() && Instant::now() < deadline {
        controller.poll_background_jobs();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!controller.runtime.jobs.umap_build_in_progress());
}
