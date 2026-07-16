#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::params;
use serde_json::Value;
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    readiness::{
        ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessFailureClassification,
        ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome, ReadinessRetryPolicy,
        ReadinessStage, ReadinessTarget, ReadinessWorkMutationOutcome, cancel_readiness_work,
        claim_readiness_target, complete_readiness_work, fail_readiness_work,
        persist_readiness_deficits, readiness_work_stats, reconcile_readiness,
        renew_readiness_lease,
    },
    scanner::complete_pending_deep_hashes,
};

use super::scheduler::{
    BudgetTracker, FairScheduler, PriorityContext, ProcessingBudgets, ProcessingLane, WorkCandidate,
};
use crate::native_app::sample_library::similarity_prep::{
    NATIVE_SIMILARITY_UMAP_VERSION, SimilarityPublicationFence, finalize_similarity_prep_if_ready,
    reset_interrupted_similarity_prep_jobs, run_similarity_prep_job,
    similarity_prep_needs_finalization,
};
use crate::native_app::waveform::cached_waveform_file_playback_ready_exists;

const SAFETY_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const DEEP_HASH_BATCH: usize = 8;
const MAX_VISIBLE_PRIORITY_PATHS: usize = 128;
const READINESS_LEASE_SECONDS: i64 = 5 * 60;
const MAX_DISCOVERED_ANALYSIS_JOBS: i64 = 256;

/// Owned runtime coordinator. All work is joined during shutdown and observes one cancel token.
pub(in crate::native_app) struct SourceProcessingSupervisor {
    shared: Arc<Shared>,
    coordinator: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub(in crate::native_app) struct SourceProcessingBudgetHandle {
    shared: Arc<Shared>,
}

pub(in crate::native_app) struct SourceProcessingBudgetPermit {
    shared: Arc<Shared>,
    permit: Option<super::scheduler::BudgetPermit>,
}

impl SourceProcessingBudgetHandle {
    pub(in crate::native_app) fn acquire_scan(
        &self,
        source_id: &str,
    ) -> SourceProcessingBudgetPermit {
        let mut budgets = self.shared.budgets();
        loop {
            if let Some(permit) = budgets.try_acquire(source_id, ProcessingLane::Scan) {
                return SourceProcessingBudgetPermit {
                    shared: Arc::clone(&self.shared),
                    permit: Some(permit),
                };
            }
            budgets = self
                .shared
                .budget_wake
                .wait(budgets)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }
}

impl Drop for SourceProcessingBudgetPermit {
    fn drop(&mut self) {
        if let Some(permit) = self.permit.take() {
            self.shared.budgets().release(permit);
            self.shared.budget_wake.notify_all();
            self.shared.control().wake("external_budget_released");
            self.shared.wake.notify_one();
        }
    }
}

impl SourceProcessingSupervisor {
    pub(in crate::native_app) fn start(sources: Vec<SampleSource>) -> Self {
        Self::start_with_playback_state(sources, false)
    }

    fn start_with_playback_state(sources: Vec<SampleSource>, playback_active: bool) -> Self {
        let shared = Arc::new(Shared::new(sources));
        shared.control().playback_active = playback_active;
        let thread_shared = Arc::clone(&shared);
        let coordinator = thread::Builder::new()
            .name(String::from("wavecrate-source-supervisor"))
            .spawn(move || run_coordinator(thread_shared))
            .expect("spawn source processing supervisor");
        Self {
            shared,
            coordinator: Some(coordinator),
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn dormant() -> Self {
        Self {
            shared: Arc::new(Shared::new(Vec::new())),
            coordinator: None,
        }
    }

    pub(in crate::native_app) fn replace_sources(&self, sources: Vec<SampleSource>) {
        self.shared.work_cancel.store(true, Ordering::Release);
        let mut control = self.shared.control();
        control.sources = sources_by_id(sources);
        control.priority.immediate.clear();
        control.priority.visible.clear();
        control.priority.immediate_paths.clear();
        control.priority.visible_paths.clear();
        control.wake("configured_sources_changed");
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn budget_handle(&self) -> SourceProcessingBudgetHandle {
        SourceProcessingBudgetHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(in crate::native_app) fn wake_source(&self, source_id: &str, reason: &'static str) {
        let mut control = self.shared.control();
        if control.sources.contains_key(source_id) {
            self.shared.work_cancel.store(true, Ordering::Release);
            control.wake(reason);
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_selected_source(&self, source_id: Option<&str>) {
        let mut control = self.shared.control();
        let selected = source_id.map(str::to_string);
        if control.priority.selected_source != selected {
            control.priority.selected_source = selected;
            control.wake("selected_source_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn prioritize_path(
        &self,
        source_id: &str,
        relative_path: &str,
        immediate: bool,
    ) {
        let mut control = self.shared.control();
        let key = (source_id.to_string(), relative_path.to_string());
        let priorities = if immediate {
            control.priority.immediate_paths.clear();
            &mut control.priority.immediate_paths
        } else {
            &mut control.priority.visible_paths
        };
        if priorities.insert(key) {
            control.wake("interactive_path_priority");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_visible_paths<I>(&self, paths: I)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let visible_paths = paths.into_iter().take(MAX_VISIBLE_PRIORITY_PATHS).collect();
        let mut control = self.shared.control();
        if control.priority.visible_paths != visible_paths {
            control.priority.visible_paths = visible_paths;
            control.wake("visible_paths_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_current_folder(&self, source_id: &str, relative_path: &str) {
        let mut control = self.shared.control();
        let current = Some((source_id.to_string(), relative_path.to_string()));
        if control.priority.current_folder != current {
            control.priority.current_folder = current;
            control.wake("current_folder_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_playback_active(&self, active: bool) {
        let mut control = self.shared.control();
        if control.playback_active != active {
            control.playback_active = active;
            self.shared.work_cancel.store(active, Ordering::Release);
            control.wake(if active {
                "playback_pause"
            } else {
                "playback_resume"
            });
            self.shared.wake.notify_all();
        }
    }

    pub(in crate::native_app) fn shutdown(&mut self) -> Value {
        let started_at = Instant::now();
        self.shared.cancel.store(true, Ordering::Release);
        self.shared.work_cancel.store(true, Ordering::Release);
        {
            let mut control = self.shared.control();
            control.shutdown = true;
            control.wake("shutdown");
        }
        self.shared.wake.notify_all();
        let joined = self
            .coordinator
            .take()
            .is_none_or(|coordinator| coordinator.join().is_ok());
        let telemetry = self.shared.telemetry();
        serde_json::json!({
            "joined": joined,
            "elapsed_ms": started_at.elapsed().as_secs_f64() * 1_000.0,
            "sweeps": telemetry.sweeps,
            "claimed": telemetry.claimed,
            "completed": telemetry.completed,
            "failed": telemetry.failed,
            "retried": telemetry.retried,
            "stale": telemetry.stale,
            "cancelled": telemetry.cancelled,
            "contention": telemetry.contention,
            "max_queue_depth": telemetry.max_queue_depth,
            "queue_depth": telemetry.queue_depth,
            "oldest_job_age_seconds": telemetry.oldest_job_age_seconds,
            "retries_due": telemetry.retries_due,
            "readiness_queue_depth": telemetry.readiness_queue_depth,
        })
    }
}

impl Drop for SourceProcessingSupervisor {
    fn drop(&mut self) {
        if self.coordinator.is_some() {
            let _ = self.shutdown();
        }
    }
}

struct Shared {
    state: Mutex<ControlState>,
    wake: Condvar,
    cancel: AtomicBool,
    work_cancel: AtomicBool,
    telemetry: Mutex<SupervisorTelemetry>,
    budgets: Mutex<BudgetTracker>,
    budget_wake: Condvar,
}

impl Shared {
    fn new(sources: Vec<SampleSource>) -> Self {
        Self {
            state: Mutex::new(ControlState {
                sources: sources_by_id(sources),
                wake_generation: 1,
                wake_reason: "startup",
                playback_active: false,
                shutdown: false,
                priority: PriorityContext::default(),
            }),
            wake: Condvar::new(),
            cancel: AtomicBool::new(false),
            work_cancel: AtomicBool::new(false),
            telemetry: Mutex::new(SupervisorTelemetry::default()),
            budgets: Mutex::new(BudgetTracker::new(ProcessingBudgets::default())),
            budget_wake: Condvar::new(),
        }
    }

    fn control(&self) -> MutexGuard<'_, ControlState> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn telemetry(&self) -> MutexGuard<'_, SupervisorTelemetry> {
        self.telemetry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn budgets(&self) -> MutexGuard<'_, BudgetTracker> {
        self.budgets
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

struct ControlState {
    sources: BTreeMap<String, SampleSource>,
    wake_generation: u64,
    wake_reason: &'static str,
    playback_active: bool,
    shutdown: bool,
    priority: PriorityContext,
}

impl ControlState {
    fn wake(&mut self, reason: &'static str) {
        self.wake_generation = self.wake_generation.wrapping_add(1);
        self.wake_reason = reason;
    }
}

#[derive(Default)]
struct SupervisorTelemetry {
    sweeps: u64,
    claimed: u64,
    completed: u64,
    failed: u64,
    retried: u64,
    stale: u64,
    cancelled: u64,
    contention: u64,
    max_queue_depth: usize,
    queue_depth: usize,
    oldest_job_age_seconds: u64,
    retries_due: usize,
    readiness_queue_depth: usize,
}

#[derive(Clone, Debug)]
enum RuntimeTask {
    DeepHash,
    LegacyAnalysis {
        job_id: i64,
    },
    FinalizeSimilarity {
        publication_fence: SimilarityPublicationFence,
    },
    Readiness(ReadinessTarget),
}

struct RuntimeCandidate {
    schedule: WorkCandidate,
    source: SampleSource,
    task: RuntimeTask,
}

#[derive(Clone, Copy, Debug, Default)]
struct SourceDiscoveryStats {
    readiness_queue_depth: usize,
    retries_due: usize,
    earliest_retry_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutionOutcome {
    Completed,
    Retried { retry_at: i64 },
    Failed,
    Stale,
    Cancelled,
    NotClaimed,
}

impl ExecutionOutcome {
    fn was_claimed(self) -> bool {
        !matches!(self, Self::NotClaimed)
    }
}

fn run_coordinator(shared: Arc<Shared>) {
    let mut observed_generation = 0;
    let mut next_retry_at = None;
    let mut scheduler = FairScheduler::default();
    let mut reset_sources = BTreeMap::<String, bool>::new();
    loop {
        let (sources, priority, playback_active, generation, reason) = {
            let mut control = shared.control();
            while !control.shutdown && control.wake_generation == observed_generation {
                let wait_duration = coordinator_wait_duration(next_retry_at, now_epoch_seconds());
                let (next, _) = shared
                    .wake
                    .wait_timeout(control, wait_duration)
                    .unwrap_or_else(|poison| poison.into_inner());
                control = next;
                if control.wake_generation == observed_generation {
                    let retry_due =
                        next_retry_at.is_some_and(|deadline| deadline <= now_epoch_seconds());
                    control.wake(if retry_due {
                        "retry_deadline"
                    } else {
                        "periodic_safety_sweep"
                    });
                }
            }
            if control.shutdown {
                break;
            }
            if !control.playback_active {
                shared.work_cancel.store(false, Ordering::Release);
            }
            (
                control.sources.values().cloned().collect::<Vec<_>>(),
                control.priority.clone(),
                control.playback_active,
                control.wake_generation,
                control.wake_reason,
            )
        };
        observed_generation = generation;
        scheduler.set_paused(playback_active);
        if playback_active {
            tracing::debug!(
                target: "wavecrate::source_processing",
                event = "source_processing.paused",
                reason,
                "Source processing paused for playback"
            );
            continue;
        }

        for source in &sources {
            let source_id = source.id.as_str().to_string();
            if !reset_sources.contains_key(&source_id) {
                let Some(permit) = shared
                    .budgets()
                    .try_acquire(&source_id, ProcessingLane::Cleanup)
                else {
                    continue;
                };
                match reset_interrupted_similarity_prep_jobs(source) {
                    Ok(reset) => {
                        reset_sources.insert(source_id, true);
                        if reset > 0 {
                            tracing::info!(
                                target: "wavecrate::source_processing",
                                source_id = source.id.as_str(),
                                reset,
                                "Recovered interrupted source jobs"
                            );
                        }
                    }
                    Err(error) => record_discovery_error(&shared, source, &error),
                }
                shared.budgets().release(permit);
                shared.budget_wake.notify_all();
            }
        }
        reset_sources
            .retain(|source_id, _| sources.iter().any(|source| source.id.as_str() == source_id));

        let sweep_started = Instant::now();
        let (mut candidates, mut source_stats) = discover_candidates(&shared, &sources);
        let discovered_stats = aggregate_source_stats(source_stats.values().copied());
        next_retry_at = discovered_stats.earliest_retry_at;
        {
            let mut telemetry = shared.telemetry();
            telemetry.sweeps = telemetry.sweeps.saturating_add(1);
            telemetry.queue_depth = candidates.len();
            telemetry.max_queue_depth = telemetry.max_queue_depth.max(telemetry.queue_depth);
            telemetry.oldest_job_age_seconds =
                oldest_job_age_seconds(&candidates, now_epoch_seconds());
            telemetry.retries_due = discovered_stats.retries_due;
            telemetry.readiness_queue_depth = discovered_stats.readiness_queue_depth;
        }
        while !candidates.is_empty() && !shared.cancel.load(Ordering::Acquire) {
            let control = shared.control();
            let interrupted =
                control.playback_active || control.wake_generation != observed_generation;
            drop(control);
            if interrupted {
                break;
            }
            let schedules = candidates
                .iter()
                .map(|candidate| candidate.schedule.clone())
                .collect::<Vec<_>>();
            let Some(index) = scheduler.choose(&schedules, &priority, &shared.budgets()) else {
                let mut telemetry = shared.telemetry();
                telemetry.contention = telemetry.contention.saturating_add(1);
                break;
            };
            let candidate = candidates.swap_remove(index);
            let Some(permit) = shared
                .budgets()
                .try_acquire(&candidate.schedule.source_id, candidate.schedule.lane)
            else {
                let mut telemetry = shared.telemetry();
                telemetry.contention = telemetry.contention.saturating_add(1);
                break;
            };
            let result = execute_candidate(&candidate, &shared.work_cancel);
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            let mut telemetry = shared.telemetry();
            let mut execution_outcome = None;
            match result {
                Ok(outcome) => {
                    execution_outcome = Some(outcome);
                    if outcome.was_claimed() {
                        telemetry.claimed = telemetry.claimed.saturating_add(1);
                    }
                    match outcome {
                        ExecutionOutcome::Completed => {
                            telemetry.completed = telemetry.completed.saturating_add(1)
                        }
                        ExecutionOutcome::Retried { retry_at } => {
                            telemetry.retried = telemetry.retried.saturating_add(1);
                            if let Some(stats) = source_stats.get_mut(candidate.source.id.as_str())
                            {
                                stats.earliest_retry_at =
                                    earliest_deadline(stats.earliest_retry_at, Some(retry_at));
                            }
                            let aggregate = aggregate_source_stats(source_stats.values().copied());
                            next_retry_at = aggregate.earliest_retry_at;
                        }
                        ExecutionOutcome::Failed => {
                            telemetry.failed = telemetry.failed.saturating_add(1)
                        }
                        ExecutionOutcome::Stale => {
                            telemetry.stale = telemetry.stale.saturating_add(1)
                        }
                        ExecutionOutcome::Cancelled => {
                            telemetry.cancelled = telemetry.cancelled.saturating_add(1)
                        }
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
            drop(telemetry);
            let source_id = candidate.source.id.as_str();
            if candidates
                .iter()
                .any(|queued| queued.source.id.as_str() == source_id)
            {
                continue;
            }
            let should_refresh = match (&candidate.task, execution_outcome) {
                (RuntimeTask::DeepHash, Some(ExecutionOutcome::Completed))
                | (RuntimeTask::LegacyAnalysis { .. }, Some(ExecutionOutcome::Completed))
                | (RuntimeTask::LegacyAnalysis { .. }, Some(ExecutionOutcome::Failed))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Completed))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Retried { .. }))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Failed)) => true,
                _ => false,
            };
            if !should_refresh {
                continue;
            }
            match discover_source_candidates(&candidate.source, now_epoch_seconds()) {
                Ok((mut refreshed, refreshed_stats)) => {
                    candidates.append(&mut refreshed);
                    source_stats.insert(source_id.to_string(), refreshed_stats);
                    let aggregate = aggregate_source_stats(source_stats.values().copied());
                    next_retry_at = aggregate.earliest_retry_at;
                    let mut telemetry = shared.telemetry();
                    telemetry.retries_due = aggregate.retries_due;
                    telemetry.readiness_queue_depth = aggregate.readiness_queue_depth;
                }
                Err(error) => record_discovery_error(&shared, &candidate.source, &error),
            }
        }
        let mut telemetry = shared.telemetry();
        telemetry.queue_depth = candidates.len();
        telemetry.oldest_job_age_seconds = oldest_job_age_seconds(&candidates, now_epoch_seconds());
        tracing::debug!(
            target: "wavecrate::source_processing",
            event = "source_processing.sweep",
            reason,
            source_count = sources.len(),
            queued = telemetry.queue_depth,
            oldest_job_age_seconds = telemetry.oldest_job_age_seconds,
            retries_due = telemetry.retries_due,
            claimed = telemetry.claimed,
            completed = telemetry.completed,
            failed = telemetry.failed,
            retried = telemetry.retried,
            stale = telemetry.stale,
            cancelled = telemetry.cancelled,
            contention = telemetry.contention,
            elapsed_ms = sweep_started.elapsed().as_secs_f64() * 1_000.0,
            "Source processing sweep complete"
        );
        drop(telemetry);
    }
}

fn discover_candidates(
    shared: &Shared,
    sources: &[SampleSource],
) -> (
    Vec<RuntimeCandidate>,
    BTreeMap<String, SourceDiscoveryStats>,
) {
    let now = now_epoch_seconds();
    let mut candidates = Vec::new();
    let mut source_stats = BTreeMap::new();
    for source in sources {
        let Some(permit) = shared
            .budgets()
            .try_acquire(source.id.as_str(), ProcessingLane::Cleanup)
        else {
            continue;
        };
        match discover_source_candidates(source, now) {
            Ok((mut source_candidates, stats)) => {
                candidates.append(&mut source_candidates);
                source_stats.insert(source.id.as_str().to_string(), stats);
            }
            Err(error) => record_discovery_error(shared, source, &error),
        }
        shared.budgets().release(permit);
        shared.budget_wake.notify_all();
    }
    (candidates, source_stats)
}

fn discover_source_candidates(
    source: &SampleSource,
    now: i64,
) -> Result<(Vec<RuntimeCandidate>, SourceDiscoveryStats), String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    let source_id = source.id.as_str();
    let mut candidates = Vec::new();
    let mut stats = SourceDiscoveryStats::default();
    let readiness_source_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM source_readiness_sources WHERE source_id = ?1)",
            [source_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if readiness_source_exists {
        let snapshot =
            reconcile_readiness(&connection, source_id, now).map_err(|error| error.to_string())?;
        persist_readiness_deficits(&mut connection, &snapshot.deficits, now)
            .map_err(|error| error.to_string())?;
        stats.readiness_queue_depth = snapshot.deficits.len();
        candidates.extend(snapshot.deficits.iter().map(|deficit| RuntimeCandidate {
            schedule: WorkCandidate::readiness(&deficit.target, now),
            source: source.clone(),
            task: RuntimeTask::Readiness(deficit.target.clone()),
        }));
        let work_stats =
            readiness_work_stats(&connection, now).map_err(|error| error.to_string())?;
        stats.retries_due = work_stats.retries_due;
        stats.earliest_retry_at = work_stats.earliest_retry_at;
        tracing::debug!(
            target: "wavecrate::source_processing",
            source_id,
            pending = work_stats.pending,
            running = work_stats.running,
            retries_due = work_stats.retries_due,
            retries_waiting = work_stats.retries_waiting,
            expired_leases = work_stats.expired_leases,
            "Readiness work reconciled"
        );
    }

    let has_unhashed: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM wav_files
                WHERE missing = 0 AND content_hash IS NULL
                LIMIT 1
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if has_unhashed {
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(source_id, ProcessingLane::Hashing, 1, now),
            source: source.clone(),
            task: RuntimeTask::DeepHash,
        });
    }
    let legacy_jobs = {
        let mut statement = connection
            .prepare(
                "SELECT id, relative_path, job_type, created_at
                 FROM analysis_jobs
                 WHERE readiness_managed = 0
                   AND status = 'pending'
                   AND job_type IN ('wav_metadata_v1', 'embedding_backfill_v1')
                 ORDER BY created_at, id
                 LIMIT ?1",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([MAX_DISCOVERED_ANALYSIS_JOBS], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    for (job_id, relative_path, job_type, created_at) in legacy_jobs {
        let lane = if job_type == "embedding_backfill_v1" {
            ProcessingLane::Embedding
        } else {
            ProcessingLane::FeatureAnalysis
        };
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::file(source_id, relative_path, lane, 2, created_at),
            source: source.clone(),
            task: RuntimeTask::LegacyAnalysis { job_id },
        });
    }
    if !readiness_source_exists
        && candidates
            .iter()
            .all(|candidate| !matches!(&candidate.task, RuntimeTask::LegacyAnalysis { .. }))
        && similarity_prep_needs_finalization(source)?
    {
        let paths_revision = connection
            .query_row(
                "SELECT COALESCE(
                    (SELECT CAST(value AS INTEGER) FROM metadata
                     WHERE key = 'wav_paths_revision_v1'),
                    0
                )",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?;
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(source_id, ProcessingLane::Finalization, 4, now),
            source: source.clone(),
            task: RuntimeTask::FinalizeSimilarity {
                publication_fence: SimilarityPublicationFence::legacy_paths_revision(
                    paths_revision,
                ),
            },
        });
    }
    Ok((candidates, stats))
}

fn execute_candidate(
    candidate: &RuntimeCandidate,
    cancel: &AtomicBool,
) -> Result<ExecutionOutcome, String> {
    let result = match &candidate.task {
        RuntimeTask::DeepHash => {
            let database_root = candidate
                .source
                .database_root()
                .map_err(|error| error.to_string())?;
            let db = SourceDatabase::open_for_background_job_with_database_root(
                &candidate.source.root,
                database_root,
            )
            .map_err(|error| error.to_string())?;
            complete_pending_deep_hashes(&db, Some(cancel), DEEP_HASH_BATCH)
                .map(|stats| {
                    if stats.hashes_computed > 0 {
                        ExecutionOutcome::Completed
                    } else {
                        ExecutionOutcome::NotClaimed
                    }
                })
                .map_err(|error| error.to_string())
        }
        RuntimeTask::LegacyAnalysis { job_id } => {
            run_similarity_prep_job(&candidate.source, *job_id, cancel, 1).map(|summary| {
                if summary.processed == 0 {
                    ExecutionOutcome::NotClaimed
                } else if summary.failed > 0 {
                    ExecutionOutcome::Failed
                } else {
                    ExecutionOutcome::Completed
                }
            })
        }
        RuntimeTask::FinalizeSimilarity { publication_fence } => {
            finalize_similarity_prep_if_ready(&candidate.source, publication_fence, cancel).map(
                |finalized| {
                    if finalized {
                        ExecutionOutcome::Completed
                    } else {
                        ExecutionOutcome::NotClaimed
                    }
                },
            )
        }
        RuntimeTask::Readiness(target) => {
            execute_readiness_target(&candidate.source, target, cancel)
        }
    };
    if cancel.load(Ordering::Acquire) {
        Ok(ExecutionOutcome::Cancelled)
    } else {
        result
    }
}

enum ReadinessExecutionOutcome {
    Complete,
    Retry(&'static str),
    Permanent(&'static str),
}

fn execute_readiness_target(
    source: &SampleSource,
    target: &ReadinessTarget,
    cancel: &AtomicBool,
) -> Result<ExecutionOutcome, String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    let now = now_epoch_seconds();
    let Some(claim) = claim_readiness_target(&mut connection, target, now, READINESS_LEASE_SECONDS)
        .map_err(|error| error.to_string())?
    else {
        return Ok(ExecutionOutcome::NotClaimed);
    };
    if cancel.load(Ordering::Acquire) {
        return cancel_claim(&mut connection, &claim, "runtime cancellation", now);
    }
    let (outcome, lease_stale) = match run_with_readiness_lease_heartbeat(
        source,
        &claim,
        cancel,
        READINESS_LEASE_SECONDS,
        |lease_cancel| run_readiness_stage(source, &connection, &claim, lease_cancel),
    ) {
        Ok(result) => result,
        Err(error) => {
            let _ = cancel_readiness_work(
                &mut connection,
                &claim,
                "readiness lease heartbeat failure",
                now_epoch_seconds(),
            );
            return Err(error);
        }
    };
    if lease_stale {
        return Ok(ExecutionOutcome::Stale);
    }
    if cancel.load(Ordering::Acquire) {
        return cancel_claim(
            &mut connection,
            &claim,
            "runtime cancellation before readiness publication",
            now_epoch_seconds(),
        );
    }
    match outcome {
        Ok(ReadinessExecutionOutcome::Complete) => {
            match complete_readiness_work(&mut connection, &claim, now_epoch_seconds())
                .map_err(|error| error.to_string())?
            {
                ArtifactPublishOutcome::Recorded => Ok(ExecutionOutcome::Completed),
                ArtifactPublishOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
            }
        }
        Ok(ReadinessExecutionOutcome::Retry(reason)) => {
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, u32::MAX)
                .expect("valid readiness retry policy");
            let outcome = fail_readiness_work(
                &mut connection,
                &claim,
                ReadinessFailureClassification::Retryable,
                reason,
                now_epoch_seconds(),
                policy,
            )
            .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
        Err(reason) => {
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, u32::MAX)
                .expect("valid readiness retry policy");
            let outcome = fail_readiness_work(
                &mut connection,
                &claim,
                ReadinessFailureClassification::Retryable,
                &reason,
                now_epoch_seconds(),
                policy,
            )
            .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
        Ok(ReadinessExecutionOutcome::Permanent(reason)) => {
            let policy =
                ReadinessRetryPolicy::new(5, 5 * 60, 1).expect("valid readiness terminal policy");
            let outcome = fail_readiness_work(
                &mut connection,
                &claim,
                ReadinessFailureClassification::Permanent,
                reason,
                now_epoch_seconds(),
                policy,
            )
            .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
    }
}

fn cancel_claim(
    connection: &mut rusqlite::Connection,
    claim: &ClaimedReadinessWork,
    reason: &str,
    now: i64,
) -> Result<ExecutionOutcome, String> {
    match cancel_readiness_work(connection, claim, reason, now)
        .map_err(|error| error.to_string())?
    {
        ReadinessWorkMutationOutcome::Recorded => Ok(ExecutionOutcome::Cancelled),
        ReadinessWorkMutationOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
    }
}

fn execution_outcome_for_failure(outcome: ReadinessFailureOutcome) -> ExecutionOutcome {
    match outcome {
        ReadinessFailureOutcome::RetryScheduled { retry_at } => {
            ExecutionOutcome::Retried { retry_at }
        }
        ReadinessFailureOutcome::RejectedStale => ExecutionOutcome::Stale,
        ReadinessFailureOutcome::Permanent
        | ReadinessFailureOutcome::Unsupported
        | ReadinessFailureOutcome::AttemptsExhausted => ExecutionOutcome::Failed,
    }
}

fn run_with_readiness_lease_heartbeat<T>(
    source: &SampleSource,
    claim: &ClaimedReadinessWork,
    external_cancel: &AtomicBool,
    lease_duration_seconds: i64,
    execute: impl FnOnce(&AtomicBool) -> T,
) -> Result<(T, bool), String> {
    let local_cancel = AtomicBool::new(external_cancel.load(Ordering::Acquire));
    let stop = AtomicBool::new(false);
    let lease_stale = AtomicBool::new(false);
    let heartbeat_error = Mutex::new(None::<String>);
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let renew_interval = Duration::from_secs((lease_duration_seconds / 3).max(1) as u64);
    let mut heartbeat_connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;

    let result = thread::scope(|scope| {
        scope.spawn(|| {
            let mut next_renewal = Instant::now() + renew_interval;
            while !stop.load(Ordering::Acquire) {
                if external_cancel.load(Ordering::Acquire) {
                    local_cancel.store(true, Ordering::Release);
                }
                if Instant::now() >= next_renewal {
                    match renew_readiness_lease(
                        &mut heartbeat_connection,
                        claim,
                        now_epoch_seconds(),
                        lease_duration_seconds,
                    ) {
                        Ok(ReadinessLeaseRenewalOutcome::Renewed { .. }) => {
                            next_renewal = Instant::now() + renew_interval;
                        }
                        Ok(ReadinessLeaseRenewalOutcome::RejectedStale) => {
                            lease_stale.store(true, Ordering::Release);
                            local_cancel.store(true, Ordering::Release);
                            return;
                        }
                        Err(error) => {
                            *heartbeat_error
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner()) =
                                Some(error.to_string());
                            local_cancel.store(true, Ordering::Release);
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
        let result = execute(&local_cancel);
        stop.store(true, Ordering::Release);
        result
    });
    if let Some(error) = heartbeat_error
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
    {
        return Err(format!("Readiness lease heartbeat failed: {error}"));
    }
    Ok((result, lease_stale.load(Ordering::Acquire)))
}

fn run_readiness_stage(
    source: &SampleSource,
    connection: &rusqlite::Connection,
    claim: &ClaimedReadinessWork,
    cancel: &AtomicBool,
) -> Result<ReadinessExecutionOutcome, String> {
    let target = claim.target();
    match target.stage {
        ReadinessStage::IndexedIdentity => {
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "indexed identity target has no relative path",
                ));
            };
            let current: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM wav_files
                        WHERE file_identity = ?1 AND path = ?2 AND missing = 0
                    )",
                    params![target.scope_id, relative_path],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            Ok(if current {
                ReadinessExecutionOutcome::Complete
            } else {
                ReadinessExecutionOutcome::Retry("indexed identity is not committed yet")
            })
        }
        ReadinessStage::PlaybackSummary => {
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "playback summary target has no relative path",
                ));
            };
            Ok(
                if cached_waveform_file_playback_ready_exists(&source.root.join(relative_path)) {
                    ReadinessExecutionOutcome::Complete
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "playback summary prerequisite is not durable yet",
                    )
                },
            )
        }
        ReadinessStage::AnalysisFeatures => {
            let current: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM analysis_cache_features
                        WHERE content_hash = ?1 AND analysis_version = ?2
                    )",
                    params![target.content_generation, target.required_version],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            Ok(if current {
                ReadinessExecutionOutcome::Complete
            } else {
                ReadinessExecutionOutcome::Retry("analysis feature prerequisite is not durable yet")
            })
        }
        ReadinessStage::EmbeddingAspects => {
            let expected_version = format!(
                "{}+{}",
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            );
            if target.required_version != expected_version {
                return Ok(ReadinessExecutionOutcome::Retry(
                    "embedding executor version does not match target",
                ));
            }
            let current: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM analysis_cache_embeddings
                        WHERE content_hash = ?1 AND model_id = ?2
                    ) AND EXISTS(
                        SELECT 1 FROM analysis_cache_aspect_descriptors
                        WHERE content_hash = ?1 AND model_id = ?3
                    )",
                    params![
                        target.content_generation,
                        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                    ],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            Ok(if current {
                ReadinessExecutionOutcome::Complete
            } else {
                ReadinessExecutionOutcome::Retry("embedding prerequisite is not durable yet")
            })
        }
        ReadinessStage::SimilarityLayout => {
            if target.required_version != NATIVE_SIMILARITY_UMAP_VERSION {
                return Ok(ReadinessExecutionOutcome::Retry(
                    "similarity finalizer version does not match target",
                ));
            }
            if cancel.load(Ordering::Acquire) {
                return Ok(ReadinessExecutionOutcome::Retry(
                    "similarity finalization cancelled",
                ));
            }
            let publication_fence = SimilarityPublicationFence::for_readiness_target(target)?;
            finalize_similarity_prep_if_ready(source, &publication_fence, cancel).map(|finalized| {
                if finalized {
                    ReadinessExecutionOutcome::Complete
                } else {
                    ReadinessExecutionOutcome::Retry("similarity prerequisites are not durable yet")
                }
            })
        }
    }
}

fn record_discovery_error(shared: &Shared, source: &SampleSource, error: &str) {
    let mut telemetry = shared.telemetry();
    telemetry.failed = telemetry.failed.saturating_add(1);
    drop(telemetry);
    tracing::warn!(
        target: "wavecrate::source_processing",
        source_id = source.id.as_str(),
        error,
        "Source processing discovery failed"
    );
}

fn sources_by_id(sources: Vec<SampleSource>) -> BTreeMap<String, SampleSource> {
    sources
        .into_iter()
        .map(|source| (source.id.as_str().to_string(), source))
        .collect()
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i64::MAX as u64) as i64
}

fn earliest_deadline(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn aggregate_source_stats(
    stats: impl IntoIterator<Item = SourceDiscoveryStats>,
) -> SourceDiscoveryStats {
    stats
        .into_iter()
        .fold(SourceDiscoveryStats::default(), |mut aggregate, source| {
            aggregate.readiness_queue_depth = aggregate
                .readiness_queue_depth
                .saturating_add(source.readiness_queue_depth);
            aggregate.retries_due = aggregate.retries_due.saturating_add(source.retries_due);
            aggregate.earliest_retry_at =
                earliest_deadline(aggregate.earliest_retry_at, source.earliest_retry_at);
            aggregate
        })
}

fn coordinator_wait_duration(next_retry_at: Option<i64>, now: i64) -> Duration {
    let retry_wait = next_retry_at.map_or(SAFETY_SWEEP_INTERVAL, |deadline| {
        Duration::from_secs(deadline.saturating_sub(now).max(0) as u64)
    });
    SAFETY_SWEEP_INTERVAL.min(retry_wait)
}

fn oldest_job_age_seconds(candidates: &[RuntimeCandidate], now: i64) -> u64 {
    candidates
        .iter()
        .map(|candidate| now.saturating_sub(candidate.schedule.enqueued_at) as u64)
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use wavecrate::sample_sources::{
        SourceId,
        readiness::{ReadinessEligibility, SourceAvailability, replace_readiness_targets},
    };

    use super::*;

    #[test]
    fn playback_pause_retains_hash_backlog_until_resume_and_shutdown_joins() {
        let (_directory, source) = unhashed_source("paused");
        let mut supervisor =
            SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);

        thread::sleep(Duration::from_millis(100));
        assert!(!source_is_hashed(&source));

        supervisor.set_playback_active(false);
        wait_until(Duration::from_secs(3), || source_is_hashed(&source));
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
    }

    #[test]
    fn removing_a_source_cancels_its_unstarted_backlog() {
        let (_directory, source) = unhashed_source("removed");
        let mut supervisor =
            SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);
        supervisor.replace_sources(Vec::new());
        supervisor.set_playback_active(false);

        thread::sleep(Duration::from_millis(150));
        assert!(!source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn production_supervisor_claims_and_completes_indexed_identity_readiness() {
        let (_directory, source) = unhashed_source("readiness");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'identity-1', content_hash = 'content-1'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("assign file identity");
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "identity-1",
            "pending.wav",
            ReadinessStage::IndexedIdentity,
            "manifest-v1",
            1,
            "content-1",
        );
        let mut targets = vec![target.clone()];
        for stage in [
            ReadinessStage::PlaybackSummary,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            let mut terminal = target.clone();
            terminal.stage = stage;
            terminal.required_version = "unsupported-v1".to_string();
            terminal.eligibility = ReadinessEligibility::Unsupported;
            targets.push(terminal);
        }
        targets.push(
            ReadinessTarget::source(
                source.id.as_str(),
                ReadinessStage::SimilarityLayout,
                "layout-v1",
                1,
                "members-1",
            )
            .with_eligibility(ReadinessEligibility::Unsupported),
        );
        replace_readiness_targets(
            &mut connection,
            source.id.as_str(),
            1,
            1,
            SourceAvailability::Active,
            &targets,
            now_epoch_seconds(),
        )
        .expect("publish readiness target");
        drop(connection);

        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(3), || {
            let database_root = source.database_root().expect("database root");
            let connection = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            )
            .expect("open readiness database");
            connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM source_readiness_artifacts
                        WHERE source_id = ?1 AND scope_id = 'identity-1'
                          AND stage = 'indexed_identity'
                    )",
                    [source.id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .expect("read readiness artifact")
        });
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
        assert_eq!(report["claimed"], 1);
        assert_eq!(report["completed"], 1);
        assert_eq!(report["retried"], 0);
        assert_eq!(report["stale"], 0);
        assert_eq!(report["readiness_queue_depth"], 0);
    }

    #[test]
    fn real_hash_execution_waits_for_shared_scan_database_budget() {
        let (_directory, source) = unhashed_source("shared-budget");
        let mut supervisor =
            SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);
        let permit = supervisor.budget_handle().acquire_scan(source.id.as_str());
        supervisor.set_playback_active(false);
        thread::sleep(Duration::from_millis(150));
        assert!(!source_is_hashed(&source));

        drop(permit);
        wait_until(Duration::from_secs(3), || source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn readiness_lease_heartbeat_keeps_long_claim_current() {
        let (_directory, source) = unhashed_source("lease-heartbeat");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'identity-lease', content_hash = 'content-lease'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("assign file identity");
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "identity-lease",
            "pending.wav",
            ReadinessStage::IndexedIdentity,
            "manifest-v1",
            1,
            "content-lease",
        );
        let mut targets = vec![target.clone()];
        for stage in [
            ReadinessStage::PlaybackSummary,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            let mut terminal = target.clone();
            terminal.stage = stage;
            terminal.eligibility = ReadinessEligibility::Unsupported;
            targets.push(terminal);
        }
        targets.push(
            ReadinessTarget::source(
                source.id.as_str(),
                ReadinessStage::SimilarityLayout,
                "layout-v1",
                1,
                "members-1",
            )
            .with_eligibility(ReadinessEligibility::Unsupported),
        );
        let now = now_epoch_seconds();
        replace_readiness_targets(
            &mut connection,
            source.id.as_str(),
            1,
            1,
            SourceAvailability::Active,
            &targets,
            now,
        )
        .expect("publish readiness targets");
        let snapshot =
            reconcile_readiness(&connection, source.id.as_str(), now).expect("reconcile readiness");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, now)
            .expect("persist readiness work");
        let claim = claim_readiness_target(&mut connection, &target, now, 2)
            .expect("claim readiness")
            .expect("claim available");
        let cancel = AtomicBool::new(false);

        let ((), stale) = run_with_readiness_lease_heartbeat(&source, &claim, &cancel, 2, |_| {
            thread::sleep(Duration::from_millis(2_500))
        })
        .expect("run with heartbeat");

        assert!(!stale);
        assert_eq!(
            complete_readiness_work(&mut connection, &claim, now_epoch_seconds())
                .expect("complete renewed claim"),
            ArtifactPublishOutcome::Recorded
        );
    }

    #[test]
    fn retry_deadline_shortens_coordinator_wait_deterministically() {
        assert_eq!(
            coordinator_wait_duration(Some(105), 100),
            Duration::from_secs(5)
        );
        assert_eq!(coordinator_wait_duration(Some(100), 100), Duration::ZERO);
        assert_eq!(
            coordinator_wait_duration(Some(200), 100),
            SAFETY_SWEEP_INTERVAL
        );
        assert_eq!(coordinator_wait_duration(None, 100), SAFETY_SWEEP_INTERVAL);
    }

    #[test]
    fn durable_failure_outcomes_do_not_count_as_completion() {
        assert_eq!(
            execution_outcome_for_failure(ReadinessFailureOutcome::RetryScheduled { retry_at: 5 }),
            ExecutionOutcome::Retried { retry_at: 5 }
        );
        assert_eq!(
            execution_outcome_for_failure(ReadinessFailureOutcome::RejectedStale),
            ExecutionOutcome::Stale
        );
        assert_eq!(
            execution_outcome_for_failure(ReadinessFailureOutcome::Permanent),
            ExecutionOutcome::Failed
        );
    }

    fn unhashed_source(id: &str) -> (tempfile::TempDir, SampleSource) {
        let directory = tempfile::tempdir().expect("temporary source");
        let path = directory.path().join("pending.wav");
        std::fs::write(&path, [1_u8; 64]).expect("write sample bytes");
        let source =
            SampleSource::new_with_id(SourceId::from_string(id), directory.path().to_path_buf());
        let db = source.open_db().expect("open source database");
        db.upsert_file(Path::new("pending.wav"), 64, 1)
            .expect("insert pending hash row");
        (directory, source)
    }

    fn source_is_hashed(source: &SampleSource) -> bool {
        source
            .open_db()
            .expect("open source database")
            .entry_for_path(Path::new("pending.wav"))
            .expect("read pending file")
            .and_then(|entry| entry.content_hash)
            .is_some()
    }

    fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(condition(), "condition did not become true before timeout");
    }
}
