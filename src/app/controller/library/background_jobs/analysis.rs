use super::super::analysis_jobs::{self, RunningJobInfo};
use super::*;
use crate::app::state::ProgressTaskKind;
use crate::app::state::RunningJobSnapshot;
use std::time::{SystemTime, UNIX_EPOCH};

/// Apply background analysis worker events to progress UI and follow-up queues.
pub(crate) fn handle_analysis_message(controller: &mut AppController, message: AnalysisJobMessage) {
    match message {
        AnalysisJobMessage::Progress {
            source_id,
            progress,
        } => {
            if let Some(state) = controller.runtime.similarity_prep.as_ref()
                && source_id.as_ref() != Some(&state.source_id)
            {
                return;
            }
            let selected_source = controller.selection_state.ctx.selected_source.clone();
            let mut progress = progress;
            if source_id.is_none()
                && let Some(selected_id) = selected_source.as_ref()
                && let Some(source) = controller.current_source()
                && &source.id == selected_id
                && let Ok(scoped) = analysis_jobs::current_progress_for_source(&source)
            {
                progress = scoped;
            }
            let selected_matches = match source_id.as_ref() {
                None => true,
                Some(id) => selected_source
                    .as_ref()
                    .map(|selected| selected == id)
                    .unwrap_or(false),
            };
            if let Some(source_id) = source_id.as_ref()
                && controller
                    .runtime
                    .similarity_prep
                    .as_ref()
                    .is_some_and(|state| &state.source_id == source_id)
            {
                controller.handle_similarity_analysis_progress(&progress);
            }
            if !selected_matches {
                return;
            }
            if progress.total() == 0 {
                if controller.ui.progress.task == Some(ProgressTaskKind::Analysis) {
                    controller.clear_progress();
                }
                return;
            }
            if progress.pending == 0 && progress.running == 0 {
                if let Some(source) = controller.current_source() {
                    controller.queue_analysis_failures_refresh(&source);
                    controller.ui_cache.browser.features.remove(&source.id);
                    controller.ui_cache.browser.bpm_values.remove(&source.id);
                }
                if controller.ui.progress.task == Some(ProgressTaskKind::Analysis) {
                    controller.clear_progress();
                }
                return;
            }
            if controller.ui.progress.task.is_none()
                || controller.ui.progress.task == Some(ProgressTaskKind::Analysis)
            {
                progress::ensure_progress_visible(
                    controller,
                    ProgressTaskKind::Analysis,
                    "Analyzing samples",
                    progress.total(),
                    true,
                );
                let jobs_completed = progress.completed();
                let jobs_total = progress.total();
                let samples_completed = progress.samples_completed();
                let samples_total = progress.samples_total;
                let mut detail = format!(
                    "Jobs {jobs_completed}/{jobs_total} • Samples {samples_completed}/{samples_total}"
                );
                if progress.failed > 0 {
                    detail.push_str(&format!(" • {} failed", progress.failed));
                }
                let running_jobs = if let Some(source) = controller.current_source() {
                    analysis_jobs::current_running_jobs_for_source(&source, 3)
                        .ok()
                        .map(|jobs: Vec<RunningJobInfo>| {
                            let now_epoch = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .ok()
                                .map(|duration| duration.as_secs() as i64);
                            let stale_after = analysis_jobs::stale_running_job_seconds();
                            jobs.into_iter()
                                .map(|job| {
                                    let label =
                                        analysis_jobs::parse_sample_id(job.sample_id.as_str())
                                            .ok()
                                            .map(|(_, path): (String, PathBuf)| {
                                                path.to_string_lossy().to_string()
                                            })
                                            .unwrap_or(job.sample_id);
                                    RunningJobSnapshot::from_heartbeat(
                                        label,
                                        job.last_heartbeat_at,
                                        Some(stale_after),
                                        now_epoch,
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                controller.ui.progress.set_analysis_snapshot(Some(
                    crate::app::state::AnalysisProgressSnapshot {
                        pending: progress.pending,
                        running: progress.running,
                        failed: progress.failed,
                        samples_completed,
                        samples_total,
                        running_jobs,
                        stale_after_secs: Some(analysis_jobs::stale_running_job_seconds()),
                    },
                ));
                progress::update_progress_totals(
                    controller,
                    ProgressTaskKind::Analysis,
                    progress.total(),
                    progress.completed(),
                    Some(detail),
                );
            }
        }
        AnalysisJobMessage::EnqueueFinished { inserted, progress } => {
            controller.runtime.analysis.resume();
            if inserted > 0 {
                controller.set_status(format!("Queued {inserted} analysis jobs"), StatusTone::Info);
            }
            if let Some(source_id) = controller.selection_state.ctx.selected_source.clone() {
                controller.ui_cache.browser.features.remove(&source_id);
            }
            let _ = controller
                .runtime
                .jobs
                .message_sender()
                .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                    source_id: controller.selection_state.ctx.selected_source.clone(),
                    progress,
                }));
        }
        AnalysisJobMessage::EnqueueFailed(err) => {
            controller.set_status(format!("Analysis enqueue failed: {err}"), StatusTone::Error);
        }
        AnalysisJobMessage::EmbeddingBackfillEnqueueFinished { inserted, progress } => {
            controller.runtime.analysis.resume();
            if inserted > 0 {
                controller.set_status(
                    format!("Queued {inserted} embedding backfill jobs"),
                    StatusTone::Info,
                );
            }
            let _ = controller
                .runtime
                .jobs
                .message_sender()
                .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                    source_id: controller.selection_state.ctx.selected_source.clone(),
                    progress,
                }));
        }
        AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err) => {
            controller.set_status(
                format!("Embedding backfill enqueue failed: {err}"),
                StatusTone::Error,
            );
        }
        AnalysisJobMessage::DurationsUpdated { source_id, updated } => {
            if updated > 0 {
                controller.ui_cache.browser.features.remove(&source_id);
                controller.ui_cache.browser.durations.remove(&source_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::analysis_jobs::AnalysisProgress;
    use crate::app::controller::jobs::JobMessage;
    use crate::app::controller::state::cache::FeatureCache;
    use crate::app::controller::test_support::dummy_controller;
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

        assert_eq!(controller.ui.progress.task, Some(ProgressTaskKind::Analysis));
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
    fn enqueue_finished_invalidates_feature_cache_and_queues_follow_up_progress() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.ui_cache.browser.features.insert(
            source.id.clone(),
            FeatureCache { rows: Vec::new() },
        );

        let progress = sample_progress();
        handle_analysis_message(
            &mut controller,
            AnalysisJobMessage::EnqueueFinished {
                inserted: 2,
                progress,
            },
        );

        assert_eq!(controller.ui.status.text, "Queued 2 analysis jobs");
        assert!(!controller.ui_cache.browser.features.contains_key(&source.id));
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
    fn durations_updated_invalidates_cached_durations_and_features() {
        let (mut controller, source) = dummy_controller();
        let source_id = source.id.clone();
        controller.ui_cache.browser.features.insert(
            source_id.clone(),
            FeatureCache { rows: Vec::new() },
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

        assert!(!controller.ui_cache.browser.features.contains_key(&source_id));
        assert!(!controller.ui_cache.browser.durations.contains_key(&source_id));
    }
}
