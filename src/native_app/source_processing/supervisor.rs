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

use serde_json::Value;
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    readiness::{persist_readiness_deficits, readiness_work_stats, reconcile_readiness},
    scanner::complete_pending_deep_hashes,
};

use super::scheduler::{
    BudgetTracker, FairScheduler, PriorityContext, ProcessingBudgets, ProcessingLane, WorkCandidate,
};
use crate::native_app::sample_library::similarity_prep::{
    finalize_similarity_prep_if_ready, reset_interrupted_similarity_prep_jobs,
    run_similarity_prep_job_batch, similarity_prep_needs_finalization,
};

const SAFETY_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const LEGACY_ANALYSIS_BATCH: usize = 8;
const DEEP_HASH_BATCH: usize = 8;
const MAX_VISIBLE_PRIORITY_PATHS: usize = 128;

/// Owned runtime coordinator. All work is joined during shutdown and observes one cancel token.
pub(in crate::native_app) struct SourceProcessingSupervisor {
    shared: Arc<Shared>,
    coordinator: Option<JoinHandle<()>>,
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
    cancelled: u64,
    contention: u64,
    max_queue_depth: usize,
    queue_depth: usize,
    oldest_job_age_seconds: u64,
    retries_due: usize,
    readiness_queue_depth: usize,
}

#[derive(Clone, Copy, Debug)]
enum RuntimeTask {
    DeepHash,
    LegacyAnalysis,
    FinalizeSimilarity,
}

struct RuntimeCandidate {
    schedule: WorkCandidate,
    source: SampleSource,
    task: RuntimeTask,
}

fn run_coordinator(shared: Arc<Shared>) {
    let mut observed_generation = 0;
    let mut scheduler = FairScheduler::default();
    let mut budgets = BudgetTracker::new(ProcessingBudgets::default());
    let mut reset_sources = BTreeMap::<String, bool>::new();
    loop {
        let (sources, priority, playback_active, generation, reason) = {
            let mut control = shared.control();
            while !control.shutdown && control.wake_generation == observed_generation {
                let (next, _) = shared
                    .wake
                    .wait_timeout(control, SAFETY_SWEEP_INTERVAL)
                    .unwrap_or_else(|poison| poison.into_inner());
                control = next;
                if control.wake_generation == observed_generation {
                    control.wake("periodic_safety_sweep");
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
            }
        }
        reset_sources
            .retain(|source_id, _| sources.iter().any(|source| source.id.as_str() == source_id));

        let sweep_started = Instant::now();
        let (mut candidates, retries_due, readiness_queue_depth) =
            discover_candidates(&shared, &sources);
        {
            let mut telemetry = shared.telemetry();
            telemetry.sweeps = telemetry.sweeps.saturating_add(1);
            telemetry.queue_depth = candidates.len() + readiness_queue_depth;
            telemetry.max_queue_depth = telemetry.max_queue_depth.max(telemetry.queue_depth);
            telemetry.oldest_job_age_seconds =
                oldest_job_age_seconds(&candidates, now_epoch_seconds());
            telemetry.retries_due = retries_due;
            telemetry.readiness_queue_depth = readiness_queue_depth;
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
            let Some(index) = scheduler.choose(&schedules, &priority, &budgets) else {
                let mut telemetry = shared.telemetry();
                telemetry.contention = telemetry.contention.saturating_add(1);
                break;
            };
            let candidate = candidates.swap_remove(index);
            let Some(permit) =
                budgets.try_acquire(&candidate.schedule.source_id, candidate.schedule.lane)
            else {
                let mut telemetry = shared.telemetry();
                telemetry.contention = telemetry.contention.saturating_add(1);
                break;
            };
            {
                let mut telemetry = shared.telemetry();
                telemetry.claimed = telemetry.claimed.saturating_add(1);
            }
            let result = execute_candidate(&candidate, &shared.work_cancel);
            budgets.release(permit);
            let mut telemetry = shared.telemetry();
            match result {
                _ if shared.work_cancel.load(Ordering::Acquire) => {
                    telemetry.cancelled = telemetry.cancelled.saturating_add(1);
                    tracing::debug!(
                        target: "wavecrate::source_processing",
                        source_id = candidate.source.id.as_str(),
                        task = ?candidate.task,
                        "Source work yielded to playback or source reconfiguration"
                    );
                }
                Ok(()) => telemetry.completed = telemetry.completed.saturating_add(1),
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
            (candidates, _, _) = discover_candidates(&shared, &sources);
        }
        let mut telemetry = shared.telemetry();
        telemetry.queue_depth = candidates.len() + telemetry.readiness_queue_depth;
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
) -> (Vec<RuntimeCandidate>, usize, usize) {
    let now = now_epoch_seconds();
    let mut candidates = Vec::new();
    let mut readiness_queue_depth = 0;
    let mut retries_due = 0;
    for source in sources {
        match discover_source_candidates(source, now) {
            Ok((mut source_candidates, source_readiness_depth, source_retries_due)) => {
                candidates.append(&mut source_candidates);
                readiness_queue_depth += source_readiness_depth;
                retries_due += source_retries_due;
            }
            Err(error) => record_discovery_error(shared, source, &error),
        }
    }
    (candidates, retries_due, readiness_queue_depth)
}

fn discover_source_candidates(
    source: &SampleSource,
    now: i64,
) -> Result<(Vec<RuntimeCandidate>, usize, usize), String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    let source_id = source.id.as_str();
    let mut readiness_queue_depth = 0;
    let mut retries_due = 0;
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
        readiness_queue_depth = snapshot.deficits.len();
        let stats = readiness_work_stats(&connection, now).map_err(|error| error.to_string())?;
        retries_due = stats.retries_due;
        tracing::debug!(
            target: "wavecrate::source_processing",
            source_id,
            pending = stats.pending,
            running = stats.running,
            retries_due = stats.retries_due,
            retries_waiting = stats.retries_waiting,
            expired_leases = stats.expired_leases,
            "Readiness work reconciled"
        );
    }

    let mut candidates = Vec::new();
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
    let legacy_analysis_created_at: Option<i64> = connection
        .query_row(
            "SELECT MIN(created_at) FROM analysis_jobs
             WHERE readiness_managed = 0
               AND status = 'pending'
               AND job_type IN ('wav_metadata_v1', 'embedding_backfill_v1')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if let Some(created_at) = legacy_analysis_created_at {
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(
                source_id,
                ProcessingLane::FeatureAnalysis,
                2,
                created_at,
            ),
            source: source.clone(),
            task: RuntimeTask::LegacyAnalysis,
        });
    } else if similarity_prep_needs_finalization(source)? {
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(source_id, ProcessingLane::Finalization, 4, now),
            source: source.clone(),
            task: RuntimeTask::FinalizeSimilarity,
        });
    }
    Ok((candidates, readiness_queue_depth, retries_due))
}

fn execute_candidate(candidate: &RuntimeCandidate, cancel: &AtomicBool) -> Result<(), String> {
    match candidate.task {
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
                .map(drop)
                .map_err(|error| error.to_string())
        }
        RuntimeTask::LegacyAnalysis => {
            run_similarity_prep_job_batch(&candidate.source, LEGACY_ANALYSIS_BATCH, cancel)
                .map(drop)
        }
        RuntimeTask::FinalizeSimilarity => {
            finalize_similarity_prep_if_ready(&candidate.source).map(drop)
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

    use wavecrate::sample_sources::SourceId;

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
