#[cfg(test)]
use super::execute_synthetic_candidate_for_profile;
use super::{
    Arc, AtomicBool, BTreeMap, BTreeSet, CandidateInvalidationScope, ExecutionOutcome,
    FairScheduler, Instant, Ordering, PROGRESS_REFRESH_INTERVAL, ReadinessStage, ReadinessTarget,
    RuntimeCandidate, RuntimeTask, SIMILARITY_SCORE_REFRESH_INTERVAL, Shared, SourceDiscoveryStats,
    SourceProcessingLifecycle, advance_source_progress, aggregate_source_stats,
    candidate_invalidation_scope, clear_satisfied_manifest_audit_request, earliest_deadline,
    execute_candidate, progress_refresh_due, publish_similarity_readiness_refreshes,
    publish_source_processing_progress, scheduler_candidate_indices, should_requeue_cancelled,
};

pub(super) struct CoordinatorExecutionState {
    pub(super) next_retry_at: Option<i64>,
    pub(super) pending_similarity_refresh_lifecycles: BTreeSet<SourceProcessingLifecycle>,
    pub(super) last_similarity_refresh_publish_at: Option<Instant>,
    pub(super) active_progress_source: Option<String>,
    pub(super) last_progress_publish_at: Option<Instant>,
    pub(super) progress_visible: bool,
}

pub(super) fn execute_candidates(
    shared: &Arc<Shared>,
    candidates: &mut Vec<RuntimeCandidate>,
    scheduler: &mut FairScheduler,
    source_stats: &mut BTreeMap<String, SourceDiscoveryStats>,
    source_work_cancels: &BTreeMap<String, Arc<AtomicBool>>,
    state: CoordinatorExecutionState,
    #[cfg(test)] synthetic_connections: &mut BTreeMap<String, rusqlite::Connection>,
) -> CoordinatorExecutionState {
    let CoordinatorExecutionState {
        mut next_retry_at,
        mut pending_similarity_refresh_lifecycles,
        mut last_similarity_refresh_publish_at,
        mut active_progress_source,
        mut last_progress_publish_at,
        mut progress_visible,
    } = state;
    while !candidates.is_empty() && !shared.cancel.load(Ordering::Acquire) {
        let control = shared.control();
        let interrupted = !control.dirty_sources.is_empty();
        let priority = control.priority.clone();
        drop(control);
        if interrupted {
            break;
        }
        let external_scan_admitted = shared.has_external_scan_admission();
        let eligible_indices = scheduler_candidate_indices(&candidates, external_scan_admitted);
        if eligible_indices.is_empty() {
            let mut telemetry = shared.telemetry();
            telemetry.contention = telemetry.contention.saturating_add(1);
            break;
        }
        let schedules = eligible_indices
            .iter()
            .map(|index| candidates[*index].schedule.clone())
            .collect::<Vec<_>>();
        let previously_active_source = scheduler.active_source().map(str::to_string);
        let Some(schedule_index) = scheduler.choose(&schedules, &priority, &shared.budgets())
        else {
            let mut telemetry = shared.telemetry();
            telemetry.contention = telemetry.contention.saturating_add(1);
            break;
        };
        let index = eligible_indices[schedule_index];
        let candidate = candidates.swap_remove(index);
        if scheduler.active_source() != previously_active_source.as_deref() {
            tracing::info!(
                target: "wavecrate::source_processing",
                event = "source_processing.source_selected",
                previous_source_id = previously_active_source.as_deref().unwrap_or(""),
                source_id = candidate.source.id.as_str(),
                queued_for_source = candidates
                    .iter()
                    .filter(|queued| queued.source.id == candidate.source.id)
                    .count()
                    .saturating_add(1),
                queued_total = candidates.len().saturating_add(1),
                "Selected source for exclusive processing"
            );
        }
        let Some(permit) = shared
            .budgets()
            .try_acquire(&candidate.schedule.source_id, candidate.schedule.lane)
        else {
            let mut telemetry = shared.telemetry();
            telemetry.contention = telemetry.contention.saturating_add(1);
            break;
        };
        let Some(candidate_cancel) = source_work_cancels.get(candidate.source.id.as_str()) else {
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            break;
        };
        let Some(in_flight_work) =
            shared.begin_in_flight_work(candidate.source.id.as_str(), candidate_cancel)
        else {
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            candidates.push(candidate);
            break;
        };
        let lifecycle_generation = in_flight_work.lifecycle_generation;
        let progress_publish_due = last_progress_publish_at
            .is_none_or(|published_at| published_at.elapsed() >= PROGRESS_REFRESH_INTERVAL);
        if (active_progress_source.as_deref() != Some(candidate.source.id.as_str())
            || !matches!(&candidate.task, RuntimeTask::Readiness(_))
            || progress_publish_due)
            && let Some(progress) = source_stats.get(candidate.source.id.as_str()).copied()
        {
            publish_source_processing_progress(&shared, &candidate, lifecycle_generation, progress);
            active_progress_source = Some(candidate.source.id.as_str().to_string());
            last_progress_publish_at = Some(Instant::now());
            progress_visible = true;
        }
        let candidate_started = Instant::now();
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.candidate.started",
            source_id = candidate.source.id.as_str(),
            lifecycle_generation,
            task = ?candidate.task,
            lane = ?candidate.schedule.lane,
            remaining_for_source = candidates
                .iter()
                .filter(|queued| queued.source.id == candidate.source.id)
                .count(),
            "Source processing candidate started"
        );
        if matches!(&candidate.task, RuntimeTask::ManifestAudit) {
            let mut telemetry = shared.telemetry();
            telemetry.full_audits = telemetry.full_audits.saturating_add(1);
        }
        let result = if shared.synthetic_test_execution.load(Ordering::Acquire) {
            #[cfg(test)]
            {
                execute_synthetic_candidate_for_profile(
                    &candidate,
                    candidate_cancel.as_ref(),
                    synthetic_connections,
                )
            }
            #[cfg(not(test))]
            {
                unreachable!("synthetic execution is only enabled by test supervisors")
            }
        } else {
            execute_candidate(
                &candidate,
                lifecycle_generation,
                candidate_cancel.as_ref(),
                &mut |event| shared.publish_event(event),
            )
        };
        if matches!(
            result,
            Ok(ExecutionOutcome::CompletedAwaitingForegroundRefresh
                | ExecutionOutcome::FailedAwaitingForegroundRefresh)
        ) {
            shared
                .control()
                .awaiting_foreground_refresh_sources
                .insert(candidate.source.id.as_str().to_string());
        }
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.candidate.finished",
            source_id = candidate.source.id.as_str(),
            lifecycle_generation,
            task = ?candidate.task,
            outcome = ?result,
            elapsed_ms = candidate_started.elapsed().as_secs_f64() * 1_000.0,
            "Source processing candidate finished"
        );
        drop(in_flight_work);
        shared.budgets().release(permit);
        shared.budget_wake.notify_all();
        if matches!(
            &result,
            Ok(ExecutionOutcome::Completed | ExecutionOutcome::CompletedAwaitingForegroundRefresh)
        ) && matches!(&candidate.task, RuntimeTask::ManifestAudit)
        {
            clear_satisfied_manifest_audit_request(shared.as_ref(), candidate.source.id.as_str());
        }
        let mut telemetry = shared.telemetry();
        let mut execution_outcome = None;
        match result {
            Ok(outcome) => {
                execution_outcome = Some(outcome);
                if outcome == ExecutionOutcome::Completed
                    && let RuntimeTask::Readiness(target) = &candidate.task
                    && target.stage == ReadinessStage::EmbeddingAspects
                {
                    pending_similarity_refresh_lifecycles.insert(SourceProcessingLifecycle::new(
                        target.source_id.clone(),
                        lifecycle_generation,
                    ));
                }
                if outcome.was_claimed() {
                    telemetry.claimed = telemetry.claimed.saturating_add(1);
                }
                match outcome {
                    ExecutionOutcome::Completed
                    | ExecutionOutcome::CompletedAwaitingForegroundRefresh => {
                        telemetry.completed = telemetry.completed.saturating_add(1);
                        if matches!(&candidate.task, RuntimeTask::Readiness(_))
                            && let Some(progress) =
                                advance_source_progress(source_stats, candidate.source.id.as_str())
                            && progress_refresh_due(last_progress_publish_at)
                        {
                            publish_source_processing_progress(
                                &shared,
                                &candidate,
                                lifecycle_generation,
                                progress,
                            );
                            last_progress_publish_at = Some(Instant::now());
                            progress_visible = true;
                        }
                    }
                    ExecutionOutcome::Retried { retry_at } => {
                        telemetry.retried = telemetry.retried.saturating_add(1);
                        if let Some(stats) = source_stats.get_mut(candidate.source.id.as_str()) {
                            stats.earliest_retry_at =
                                earliest_deadline(stats.earliest_retry_at, Some(retry_at));
                        }
                        let aggregate = aggregate_source_stats(source_stats.values().copied());
                        next_retry_at = aggregate.earliest_retry_at;
                    }
                    ExecutionOutcome::Failed
                    | ExecutionOutcome::FailedAwaitingForegroundRefresh => {
                        telemetry.failed = telemetry.failed.saturating_add(1);
                        if matches!(&candidate.task, RuntimeTask::Readiness(_))
                            && let Some(progress) =
                                advance_source_progress(source_stats, candidate.source.id.as_str())
                            && progress_refresh_due(last_progress_publish_at)
                        {
                            publish_source_processing_progress(
                                &shared,
                                &candidate,
                                lifecycle_generation,
                                progress,
                            );
                            last_progress_publish_at = Some(Instant::now());
                            progress_visible = true;
                        }
                    }
                    ExecutionOutcome::PrerequisiteInvalidated {
                        retry_at,
                        reason: _,
                    } => {
                        telemetry.stale = telemetry.stale.saturating_add(1);
                        if let Some(stats) = source_stats.get_mut(candidate.source.id.as_str()) {
                            stats.earliest_retry_at =
                                earliest_deadline(stats.earliest_retry_at, Some(retry_at));
                        }
                        let aggregate = aggregate_source_stats(source_stats.values().copied());
                        next_retry_at = aggregate.earliest_retry_at;
                    }
                    ExecutionOutcome::Stale => telemetry.stale = telemetry.stale.saturating_add(1),
                    ExecutionOutcome::Cancelled => {
                        telemetry.cancelled = telemetry.cancelled.saturating_add(1)
                    }
                    ExecutionOutcome::Parked => {}
                    ExecutionOutcome::NotClaimed => {}
                }
            }
            Err(error) if shared.cancel.load(Ordering::Acquire) => {
                telemetry.cancelled = telemetry.cancelled.saturating_add(1);
                tracing::debug!(
                    target: "wavecrate::source_processing",
                    source_id = candidate.source.id.as_str(),
                    task = ?candidate.task,
                    error,
                    "Source work cancelled"
                );
            }
            Err(error) => {
                telemetry.failed = telemetry.failed.saturating_add(1);
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    source_id = candidate.source.id.as_str(),
                    task = ?candidate.task,
                    error,
                    "Source work failed"
                );
                break;
            }
        }
        let aggregate = aggregate_source_stats(source_stats.values().copied());
        telemetry.readiness_queue_depth = aggregate.readiness_queue_depth;
        telemetry.readiness_queue_depth_by_source = source_stats
            .iter()
            .map(|(source_id, stats)| (source_id.clone(), stats.readiness_queue_depth))
            .collect();
        drop(telemetry);
        if last_similarity_refresh_publish_at
            .is_none_or(|published_at| published_at.elapsed() >= SIMILARITY_SCORE_REFRESH_INTERVAL)
            && publish_similarity_readiness_refreshes(
                &shared,
                &mut pending_similarity_refresh_lifecycles,
            )
        {
            last_similarity_refresh_publish_at = Some(Instant::now());
        }
        let requeue_cancelled = {
            let control = shared.control();
            should_requeue_cancelled(
                execution_outcome,
                control.source_is_active(candidate.source.id.as_str()),
                control.dirty_sources.contains(candidate.source.id.as_str()),
            )
        };
        if requeue_cancelled {
            candidates.push(candidate);
            continue;
        }
        match candidate_invalidation_scope(&candidate.task, execution_outcome) {
            CandidateInvalidationScope::None => {}
            CandidateInvalidationScope::TargetScope => {
                candidates.retain(|queued| {
                    queued.source.id != candidate.source.id
                        || queued.schedule.scope_id != candidate.schedule.scope_id
                });
            }
            CandidateInvalidationScope::Source => {
                candidates.retain(|queued| queued.source.id != candidate.source.id);
            }
        }
        let source_id = candidate.source.id.as_str();
        if candidates
            .iter()
            .any(|queued| queued.source.id.as_str() == source_id)
        {
            continue;
        }
        let should_refresh = matches!(
            (&candidate.task, execution_outcome),
            (
                RuntimeTask::ManifestAudit,
                Some(ExecutionOutcome::Completed)
            ) | (RuntimeTask::ManifestAudit, Some(ExecutionOutcome::Failed))
                | (
                    RuntimeTask::Readiness(ReadinessTarget {
                        stage: ReadinessStage::IndexedIdentity
                            | ReadinessStage::AnalysisFeatures
                            | ReadinessStage::EmbeddingAspects,
                        ..
                    }),
                    Some(ExecutionOutcome::Completed),
                )
                | (
                    RuntimeTask::Readiness(_),
                    Some(ExecutionOutcome::Retried { .. })
                )
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Failed))
                | (
                    RuntimeTask::Readiness(_),
                    Some(ExecutionOutcome::PrerequisiteInvalidated { .. }),
                )
                | (
                    RuntimeTask::Readiness(_),
                    Some(ExecutionOutcome::NotClaimed)
                )
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Stale))
        );
        if !should_refresh {
            continue;
        }
        shared
            .control()
            .mark_source_dirty(source_id, "source_stage_progress");
        break;
    }
    CoordinatorExecutionState {
        next_retry_at,
        pending_similarity_refresh_lifecycles,
        last_similarity_refresh_publish_at,
        active_progress_source,
        last_progress_publish_at,
        progress_visible,
    }
}
