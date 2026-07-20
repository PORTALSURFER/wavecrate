use super::{
    BTreeMap, BTreeSet, Duration, FairScheduler, PriorityContext, RuntimeCandidate, SampleSource,
    SourceDiscoveryStats, SystemTime, UNIX_EPOCH,
};

pub(super) fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i64::MAX as u64) as i64
}

pub(super) fn earliest_deadline(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

pub(super) fn select_source_for_discovery(
    sources: &[SampleSource],
    pending_source_ids: &BTreeSet<String>,
    active_source_id: Option<&str>,
    priority: &PriorityContext,
) -> Option<String> {
    if let Some(active_source_id) = active_source_id {
        return pending_source_ids
            .contains(active_source_id)
            .then(|| active_source_id.to_string());
    }
    let prioritized = priority
        .selected_source
        .iter()
        .map(String::as_str)
        .chain(
            priority
                .current_folder
                .iter()
                .map(|(source_id, _)| source_id.as_str()),
        )
        .chain(
            priority
                .immediate_paths
                .iter()
                .map(|(source_id, _)| source_id.as_str()),
        )
        .chain(
            priority
                .visible_paths
                .iter()
                .map(|(source_id, _)| source_id.as_str()),
        )
        .find(|source_id| pending_source_ids.contains(*source_id));
    prioritized.map(str::to_string).or_else(|| {
        sources
            .iter()
            .map(|source| source.id.as_str())
            .find(|source_id| pending_source_ids.contains(*source_id))
            .map(str::to_string)
    })
}

pub(super) fn release_converged_source_owner(
    scheduler: &mut FairScheduler,
    configured_source_ids: &BTreeSet<String>,
    source_stats: &BTreeMap<String, SourceDiscoveryStats>,
    candidates: &[RuntimeCandidate],
) {
    let Some(active_source_id) = scheduler.active_source().map(str::to_string) else {
        return;
    };
    let has_runnable_candidate = candidates
        .iter()
        .any(|candidate| candidate.source.id.as_str() == active_source_id);
    let release_reason = if !configured_source_ids.contains(&active_source_id) {
        Some("source_removed")
    } else if let Some(stats) = source_stats.get(&active_source_id)
        && !has_runnable_candidate
        && stats.earliest_retry_at.is_some()
    {
        Some("waiting_for_retry")
    } else if let Some(stats) = source_stats.get(&active_source_id)
        && stats.readiness_queue_depth == 0
        && stats.earliest_retry_at.is_none()
        && !has_runnable_candidate
    {
        Some(if stats.prerequisites_blocked == 0 {
            "converged_or_terminal"
        } else {
            "terminal_prerequisite_block"
        })
    } else {
        None
    };
    let Some(reason) = release_reason else {
        return;
    };
    let Some(source_id) = scheduler.release_active_source() else {
        return;
    };
    tracing::info!(
        target: "wavecrate::source_processing",
        event = "source_processing.source_released",
        source_id,
        reason,
        "Released exclusive source processing ownership"
    );
}

pub(super) fn aggregate_source_stats(
    stats: impl IntoIterator<Item = SourceDiscoveryStats>,
) -> SourceDiscoveryStats {
    stats
        .into_iter()
        .fold(SourceDiscoveryStats::default(), |mut aggregate, source| {
            aggregate.readiness_queue_depth = aggregate
                .readiness_queue_depth
                .saturating_add(source.readiness_queue_depth);
            aggregate.prerequisites_blocked = aggregate
                .prerequisites_blocked
                .saturating_add(source.prerequisites_blocked);
            aggregate.retries_due = aggregate.retries_due.saturating_add(source.retries_due);
            aggregate.earliest_retry_at =
                earliest_deadline(aggregate.earliest_retry_at, source.earliest_retry_at);
            aggregate.progress_completed = aggregate
                .progress_completed
                .saturating_add(source.progress_completed);
            aggregate.progress_total = aggregate
                .progress_total
                .saturating_add(source.progress_total);
            aggregate
        })
}

pub(super) fn coordinator_wait_duration(
    next_retry_at: Option<i64>,
    now: i64,
    safety_wait: Duration,
) -> Duration {
    let retry_wait = next_retry_at.map_or(safety_wait, |deadline| {
        Duration::from_secs(deadline.saturating_sub(now).max(0) as u64)
    });
    safety_wait.min(retry_wait)
}

pub(super) fn oldest_job_age_seconds(candidates: &[RuntimeCandidate], now: i64) -> u64 {
    candidates
        .iter()
        .map(|candidate| now.saturating_sub(candidate.schedule.enqueued_at) as u64)
        .max()
        .unwrap_or_default()
}

pub(super) fn queue_depths_by_source(candidates: &[RuntimeCandidate]) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    for candidate in candidates {
        *depths
            .entry(candidate.source.id.as_str().to_string())
            .or_default() += 1;
    }
    depths
}
