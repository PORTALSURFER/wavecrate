use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::analysis_jobs::AnalysisProgress;
use crate::app::controller::state::cache::{FeatureCache, FeatureCacheKey};
use crate::app::controller::test_support::dummy_controller;
use crate::app_core::state::StatusTone;
use std::collections::HashMap;
use std::path::PathBuf;

fn sample_progress() -> AnalysisProgress {
    AnalysisProgress {
        pending: 2,
        running: 1,
        done: 3,
        failed: 1,
        samples_total: 5,
        samples_pending_or_running: 2,
    }
}

#[test]
fn progress_for_unselected_source_is_ignored() {
    let (mut controller, source) = dummy_controller();
    let other = SampleSource::new(source.root.join("other"));
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::Progress {
            source_id: Some(other.id),
            progress: sample_progress(),
        },
    );

    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
    assert!(controller.ui.progress.analysis.is_none());
}

#[test]
fn zero_total_progress_clears_visible_analysis_overlay() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(ProgressTaskKind::Analysis, "Analyzing samples", 4, true);

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::Progress {
            source_id: Some(source.id),
            progress: AnalysisProgress::default(),
        },
    );

    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
    assert!(controller.ui.progress.analysis.is_none());
}

#[test]
fn progress_message_updates_snapshot_and_detail() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::Progress {
            source_id: Some(source.id),
            progress: sample_progress(),
        },
    );

    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::Analysis)
    );
    assert!(controller.ui.progress.visible);
    assert_eq!(controller.ui.progress.completed, 4);
    assert_eq!(controller.ui.progress.total, 7);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Jobs 4/7 • Samples 3/5 • 1 failed")
    );
    let snapshot = controller
        .ui
        .progress
        .analysis
        .as_ref()
        .expect("analysis snapshot");
    assert_eq!(snapshot.pending, 2);
    assert_eq!(snapshot.running, 1);
    assert_eq!(snapshot.failed, 1);
    assert_eq!(snapshot.samples_completed, 3);
    assert_eq!(snapshot.samples_total, 5);
    assert!(snapshot.running_jobs.is_empty());
    assert_eq!(
        snapshot.stale_after_secs,
        Some(analysis_jobs::stale_running_job_seconds())
    );
}

#[test]
fn enqueue_finished_keeps_selected_source_feature_cache_and_queues_refresh() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new(),
        },
    );

    let progress = sample_progress();
    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::EnqueueFinished {
            inserted: 2,
            progress,
            announce: true,
        },
    );

    assert_eq!(controller.ui.status.text, "Queued 2 analysis jobs");
    assert!(controller.ui_cache.browser.features.contains_key(&source.id));
    assert!(
        controller
            .runtime
            .pending_browser_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| pending.source_id == source.id)
    );
    match controller.runtime.jobs.try_recv_message() {
        Ok(JobMessage::Analysis(AnalysisJobMessage::Progress {
            source_id,
            progress: queued,
        })) => {
            assert_eq!(source_id, Some(source.id));
            assert_eq!(queued, progress);
        }
        other => panic!("unexpected queued message: {other:?}"),
    }
}

#[test]
fn enqueue_finished_can_skip_status_announcement() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_status("Saved clip clip_selection_001.wav", StatusTone::Info);

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::EnqueueFinished {
            inserted: 1,
            progress: sample_progress(),
            announce: false,
        },
    );

    assert_eq!(
        controller.ui.status.text,
        "Saved clip clip_selection_001.wav"
    );
}

#[test]
fn durations_updated_keeps_selected_source_feature_cache_and_queues_refresh() {
    let (mut controller, source) = dummy_controller();
    let source_id = source.id.clone();
    controller.library.sources.push(source.clone());
    controller.ui_cache.browser.features.insert(
        source_id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new(),
        },
    );
    controller.ui_cache.browser.durations.insert(
        source_id.clone(),
        HashMap::from([(PathBuf::from("kick.wav"), 1.25)]),
    );

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::DurationsUpdated {
            source_id: source_id.clone(),
            updated: 1,
        },
    );

    assert!(controller.ui_cache.browser.features.contains_key(&source_id));
    assert!(
        controller
            .runtime
            .pending_browser_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| pending.source_id == source_id)
    );
    assert!(
        !controller
            .ui_cache
            .browser
            .durations
            .contains_key(&source_id)
    );
}
