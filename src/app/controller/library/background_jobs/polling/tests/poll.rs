use super::*;
use crate::app::controller::test_support::dummy_controller;

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
