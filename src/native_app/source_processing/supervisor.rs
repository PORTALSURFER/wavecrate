#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    db::META_WAV_PATHS_REVISION,
    readiness::{
        ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessFailureClassification,
        ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome, ReadinessRetryPolicy,
        ReadinessStage, ReadinessTarget, ReadinessWorkMutationOutcome, SourceAvailability,
        cancel_readiness_work, claim_readiness_target, complete_readiness_work,
        fail_readiness_work, persist_readiness_deficits_with_cancel, readiness_work_stats,
        reconcile_readiness_with_cancel, renew_readiness_lease,
        replace_readiness_targets_with_cancel,
    },
    scanner::complete_pending_deep_hash_for_path,
};

use super::scheduler::{
    BudgetTracker, FairScheduler, PriorityContext, ProcessingBudgets, ProcessingLane, WorkCandidate,
};
use crate::native_app::sample_library::similarity_prep::{
    NATIVE_SIMILARITY_UMAP_VERSION, SimilarityPublicationFence, finalize_similarity_prep_if_ready,
    reset_interrupted_similarity_prep_jobs, similarity_prep_needs_finalization,
};
use crate::native_app::waveform::{
    cached_waveform_file_audition_ready_exists, ensure_persisted_playback_summary,
};

const SAFETY_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const MAX_VISIBLE_PRIORITY_PATHS: usize = 128;
const READINESS_LEASE_SECONDS: i64 = 5 * 60;
const READINESS_MAX_ATTEMPTS: u32 = 8;
const MAX_DISCOVERED_ANALYSIS_JOBS: i64 = 256;
const READINESS_MANIFEST_VERSION: &str = "source_manifest_v1";
const READINESS_PLAYBACK_VERSION: &str = "waveform_cache_v4";
const META_READINESS_TARGET_FINGERPRINT: &str = "readiness_target_fingerprint_v1";

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
    registration_id: u64,
    cancel: Arc<AtomicBool>,
}

struct ExternalScanRegistration {
    source_id: String,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct ExternalScanState {
    admissions: BTreeMap<u64, String>,
    registrations: BTreeMap<u64, ExternalScanRegistration>,
}

struct InFlightWorkGuard<'a> {
    shared: &'a Shared,
    source_id: String,
}

impl Drop for InFlightWorkGuard<'_> {
    fn drop(&mut self) {
        let mut in_flight = self
            .shared
            .in_flight_work
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(count) = in_flight.get_mut(&self.source_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                in_flight.remove(&self.source_id);
            }
        }
        drop(in_flight);
        self.shared.in_flight_wake.notify_all();
    }
}

impl SourceProcessingBudgetHandle {
    pub(in crate::native_app) fn acquire_scan(
        &self,
        source_id: &str,
    ) -> Option<SourceProcessingBudgetPermit> {
        let (admission_id, admission_cancel) = {
            let mut control = self.shared.control();
            while !control.shutdown
                && control.source_is_active(source_id)
                && control.processing_paused()
            {
                control = self
                    .shared
                    .wake
                    .wait(control)
                    .unwrap_or_else(|poison| poison.into_inner());
            }
            if control.shutdown
                || self.shared.cancel.load(Ordering::Acquire)
                || !control.source_is_active(source_id)
            {
                return None;
            }
            let admission_cancel = Arc::clone(&control.source_work_cancels[source_id]);
            if admission_cancel.load(Ordering::Acquire) {
                return None;
            }
            let admission_id = self
                .shared
                .next_external_scan_id
                .fetch_add(1, Ordering::Relaxed);
            let mut external_scans = self.shared.external_scans();
            external_scans
                .admissions
                .insert(admission_id, source_id.to_string());
            (admission_id, admission_cancel)
        };
        loop {
            let mut budgets = self.shared.budgets();
            if let Some(permit) = budgets.try_acquire(source_id, ProcessingLane::Scan) {
                drop(budgets);
                let cancel = Arc::new(AtomicBool::new(false));
                let control = self.shared.control();
                let mut external_scans = self.shared.external_scans();
                external_scans.admissions.remove(&admission_id);
                if control.shutdown
                    || self.shared.cancel.load(Ordering::Acquire)
                    || !control.source_is_active(source_id)
                    || control.processing_paused()
                    || admission_cancel.load(Ordering::Acquire)
                {
                    drop(external_scans);
                    drop(control);
                    self.shared.external_scan_wake.notify_all();
                    self.shared.budgets().release(permit);
                    self.shared.budget_wake.notify_all();
                    return None;
                }
                external_scans.registrations.insert(
                    admission_id,
                    ExternalScanRegistration {
                        source_id: source_id.to_string(),
                        cancel: Arc::clone(&cancel),
                    },
                );
                drop(external_scans);
                drop(control);
                self.shared.external_scan_wake.notify_all();
                let permit = SourceProcessingBudgetPermit {
                    shared: Arc::clone(&self.shared),
                    permit: Some(permit),
                    registration_id: admission_id,
                    cancel,
                };
                if permit.should_cancel_now() {
                    permit.cancel.store(true, Ordering::Release);
                }
                return Some(permit);
            }
            drop(
                self.shared
                    .budget_wake
                    .wait(budgets)
                    .unwrap_or_else(|poison| poison.into_inner()),
            );
            let control = self.shared.control();
            let unavailable = control.shutdown
                || self.shared.cancel.load(Ordering::Acquire)
                || !control.source_is_active(source_id)
                || control.processing_paused()
                || admission_cancel.load(Ordering::Acquire);
            drop(control);
            if unavailable {
                self.shared.finish_external_scan_admission(admission_id);
                return None;
            }
        }
    }
}

impl SourceProcessingBudgetPermit {
    pub(in crate::native_app) fn cancel_token(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    fn should_cancel_now(&self) -> bool {
        if self.shared.cancel.load(Ordering::Acquire) {
            return true;
        }
        let control = self.shared.control();
        control.shutdown
            || control.processing_paused()
            || self
                .permit
                .as_ref()
                .is_some_and(|permit| !control.source_is_active(permit.source_id()))
    }
}

impl Drop for SourceProcessingBudgetPermit {
    fn drop(&mut self) {
        self.shared
            .external_scans()
            .registrations
            .remove(&self.registration_id);
        self.shared.external_scan_wake.notify_all();
        if let Some(permit) = self.permit.take() {
            let source_id = permit.source_id().to_string();
            self.shared.budgets().release(permit);
            self.shared.budget_wake.notify_all();
            let mut control = self.shared.control();
            if control.source_is_active(&source_id) {
                control.cancel_source_work(&source_id);
                control.mark_source_dirty(&source_id, "external_source_work_committed");
            } else {
                control.notify("external_budget_released");
            }
            drop(control);
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

    pub(in crate::native_app) fn replace_sources(
        &self,
        sources: Vec<SampleSource>,
    ) -> Result<(), String> {
        let _replacement = self
            .shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let sources = sources_by_id(sources);
        let mut control = self.shared.control();
        if source_maps_match(&control.sources, &sources) {
            if !control.quarantined_sources.is_empty() {
                control.quarantined_sources.clear();
                control.reset_source_work_tokens();
                control.mark_all_sources_dirty("configured_sources_reactivated");
                drop(control);
                self.shared.budget_wake.notify_all();
                self.shared.wake.notify_all();
            }
            return Ok(());
        }
        let changed_source_ids = control
            .sources
            .iter()
            .filter_map(|(source_id, current)| {
                let changed = sources
                    .get(source_id)
                    .is_none_or(|replacement| !source_descriptors_match(current, replacement));
                changed.then(|| source_id.clone())
            })
            .chain(sources.iter().filter_map(|(source_id, replacement)| {
                let changed = control
                    .sources
                    .get(source_id)
                    .is_none_or(|current| !source_descriptors_match(current, replacement));
                changed.then(|| source_id.clone())
            }))
            .collect::<std::collections::BTreeSet<_>>();
        let retired_sources = changed_source_ids
            .iter()
            .filter_map(|source_id| control.sources.get(source_id).cloned())
            .collect::<Vec<_>>();
        for source_id in &changed_source_ids {
            if let Some(cancel) = control.source_work_cancels.get(source_id) {
                cancel.store(true, Ordering::Release);
            }
        }
        drop(control);
        self.shared.cancel_external_scans(|registration| {
            changed_source_ids.contains(&registration.source_id)
        });
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_all();
        self.shared
            .wait_for_external_scans_for_sources(&changed_source_ids);
        self.shared.wait_for_in_flight_sources(&changed_source_ids);
        for source in retired_sources {
            if let Err(error) = fence_retired_source_readiness(&source) {
                let mut control = self.shared.control();
                for source_id in &changed_source_ids {
                    if let Some(cancel) = control.source_work_cancels.get(source_id) {
                        cancel.store(true, Ordering::Release);
                    }
                    if control.sources.contains_key(source_id) {
                        control.quarantined_sources.insert(source_id.clone());
                    }
                    control.dirty_sources.remove(source_id);
                }
                control.notify("configured_sources_retirement_quarantined");
                drop(control);
                self.shared.budget_wake.notify_all();
                self.shared.wake.notify_all();
                return Err(format!(
                    "Persist retired source readiness fence for {} failed: {error}",
                    source.id
                ));
            }
        }

        let mut control = self.shared.control();
        let mut source_work_cancels = BTreeMap::new();
        for (source_id, source) in &sources {
            let cancel = control
                .sources
                .get(source_id)
                .filter(|current| source_descriptors_match(current, source))
                .and_then(|_| control.source_work_cancels.get(source_id))
                .cloned()
                .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
            source_work_cancels.insert(source_id.clone(), cancel);
        }
        control.sources = sources;
        control.source_work_cancels = source_work_cancels;
        control.quarantined_sources.clear();
        let retained_source_ids = control.sources.keys().cloned().collect::<BTreeSet<_>>();
        control
            .dirty_sources
            .retain(|source_id| retained_source_ids.contains(source_id));
        control.dirty_sources.extend(
            changed_source_ids
                .iter()
                .filter(|source_id| retained_source_ids.contains(*source_id))
                .cloned(),
        );
        control.priority.immediate.retain(|priority| {
            retained_source_ids.contains(&priority.source_id)
                && !changed_source_ids.contains(&priority.source_id)
        });
        control.priority.visible.retain(|priority| {
            retained_source_ids.contains(&priority.source_id)
                && !changed_source_ids.contains(&priority.source_id)
        });
        control.priority.immediate_paths.retain(|(source_id, _)| {
            retained_source_ids.contains(source_id) && !changed_source_ids.contains(source_id)
        });
        control.priority.visible_paths.retain(|(source_id, _)| {
            retained_source_ids.contains(source_id) && !changed_source_ids.contains(source_id)
        });
        if control
            .priority
            .selected_source
            .as_ref()
            .is_some_and(|source_id| {
                !retained_source_ids.contains(source_id) || changed_source_ids.contains(source_id)
            })
        {
            control.priority.selected_source = None;
        }
        if control
            .priority
            .current_folder
            .as_ref()
            .is_some_and(|(source_id, _)| {
                !retained_source_ids.contains(source_id) || changed_source_ids.contains(source_id)
            })
        {
            control.priority.current_folder = None;
        }
        control.notify("configured_sources_changed");
        drop(control);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_all();
        Ok(())
    }

    /// Admit a newly configured source before its first external scan starts.
    ///
    /// This deliberately only grows the configured set. Full replacement may
    /// fence retired sources and wait for their in-flight work, so it must not
    /// run from the UI scan-launch path.
    pub(in crate::native_app) fn register_source_for_scan(
        &self,
        source: SampleSource,
    ) -> Result<(), String> {
        let _replacement = match self.shared.source_replacement.try_lock() {
            Ok(replacement) => replacement,
            Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err("Configured sources are currently being replaced".to_string());
            }
        };
        let source_id = source.id.as_str().to_string();
        let mut control = self.shared.control();
        if control.shutdown || self.shared.cancel.load(Ordering::Acquire) {
            return Err("Source processing supervisor is shutting down".to_string());
        }
        if let Some(current) = control.sources.get(&source_id) {
            if !source_descriptors_match(current, &source) {
                return Err(format!(
                    "Source {source_id} is already registered with a different descriptor"
                ));
            }
            if control.quarantined_sources.remove(&source_id) {
                control
                    .source_work_cancels
                    .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
                control.mark_source_dirty(&source_id, "source_scan_registration_reactivated");
                drop(control);
                self.shared.budget_wake.notify_all();
                self.shared.wake.notify_one();
            }
            return Ok(());
        }

        control.sources.insert(source_id.clone(), source);
        control
            .source_work_cancels
            .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
        control.mark_source_dirty(&source_id, "source_registered_for_scan");
        drop(control);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_one();
        Ok(())
    }

    pub(in crate::native_app) fn budget_handle(&self) -> SourceProcessingBudgetHandle {
        SourceProcessingBudgetHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(in crate::native_app) fn wake_source(&self, source_id: &str, reason: &'static str) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.cancel_source_work(source_id);
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared
            .cancel_external_scans(|registration| registration.source_id == source_id);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn set_selected_source(&self, source_id: Option<&str>) {
        let mut control = self.shared.control();
        let selected = source_id.map(str::to_string);
        if control.priority.selected_source != selected {
            control.priority.selected_source = selected;
            control.notify("selected_source_changed");
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
            control.notify("interactive_path_priority");
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
            control.notify("visible_paths_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_current_folder(&self, source_id: &str, relative_path: &str) {
        let mut control = self.shared.control();
        let current = Some((source_id.to_string(), relative_path.to_string()));
        if control.priority.current_folder != current {
            control.priority.current_folder = current;
            control.notify("current_folder_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_playback_active(&self, active: bool) {
        let mut control = self.shared.control();
        if control.playback_active != active {
            control.playback_active = active;
            if active {
                control.cancel_all_source_work();
            } else if !control.foreground_active {
                control.reset_source_work_tokens();
            }
            if active {
                control.notify("playback_pause");
            } else {
                control.notify("playback_resume");
            }
            drop(control);
            if active {
                self.shared.cancel_external_scans(|_| true);
            }
            self.shared.budget_wake.notify_all();
            self.shared.wake.notify_all();
        }
    }

    pub(in crate::native_app) fn set_foreground_activity(&self, active: bool) {
        let mut control = self.shared.control();
        if control.foreground_active == active {
            return;
        }
        control.foreground_active = active;
        if active {
            control.cancel_all_source_work();
        } else if !control.playback_active {
            control.reset_source_work_tokens();
        }
        if active {
            control.notify("foreground_activity_pause");
        } else {
            control.notify("foreground_activity_resume");
        }
        drop(control);
        if active {
            self.shared.cancel_external_scans(|_| true);
        }
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_all();
    }

    pub(in crate::native_app) fn shutdown(&mut self) -> Value {
        let started_at = Instant::now();
        self.shared.cancel.store(true, Ordering::Release);
        self.shared.cancel_external_scans(|_| true);
        {
            let mut control = self.shared.control();
            control.cancel_all_source_work();
            control.shutdown = true;
            control.notify("shutdown");
        }
        self.shared.wake.notify_all();
        self.shared.budget_wake.notify_all();
        let joined = self
            .coordinator
            .take()
            .is_none_or(|coordinator| coordinator.join().is_ok());
        self.shared.wait_for_external_scans();
        let telemetry = self.shared.telemetry();
        serde_json::json!({
            "joined": joined,
            "external_scans_joined": true,
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
            "source_discoveries": telemetry.source_discoveries,
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
    source_replacement: Mutex<()>,
    state: Mutex<ControlState>,
    wake: Condvar,
    cancel: AtomicBool,
    telemetry: Mutex<SupervisorTelemetry>,
    budgets: Mutex<BudgetTracker>,
    budget_wake: Condvar,
    external_scans: Mutex<ExternalScanState>,
    external_scan_wake: Condvar,
    next_external_scan_id: AtomicU64,
    in_flight_work: Mutex<BTreeMap<String, usize>>,
    in_flight_wake: Condvar,
}

impl Shared {
    fn new(sources: Vec<SampleSource>) -> Self {
        let sources = sources_by_id(sources);
        let source_work_cancels = sources
            .keys()
            .map(|source_id| (source_id.clone(), Arc::new(AtomicBool::new(false))))
            .collect();
        let dirty_sources = sources.keys().cloned().collect();
        Self {
            source_replacement: Mutex::new(()),
            state: Mutex::new(ControlState {
                sources,
                source_work_cancels,
                dirty_sources,
                quarantined_sources: BTreeSet::new(),
                wake_generation: 1,
                wake_reason: "startup",
                playback_active: false,
                foreground_active: false,
                shutdown: false,
                priority: PriorityContext::default(),
            }),
            wake: Condvar::new(),
            cancel: AtomicBool::new(false),
            telemetry: Mutex::new(SupervisorTelemetry::default()),
            budgets: Mutex::new(BudgetTracker::new(ProcessingBudgets::default())),
            budget_wake: Condvar::new(),
            external_scans: Mutex::new(ExternalScanState::default()),
            external_scan_wake: Condvar::new(),
            next_external_scan_id: AtomicU64::new(1),
            in_flight_work: Mutex::new(BTreeMap::new()),
            in_flight_wake: Condvar::new(),
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

    fn external_scans(&self) -> MutexGuard<'_, ExternalScanState> {
        self.external_scans
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn cancel_external_scans(&self, should_cancel: impl Fn(&ExternalScanRegistration) -> bool) {
        for registration in self.external_scans().registrations.values() {
            if should_cancel(registration) {
                registration.cancel.store(true, Ordering::Release);
            }
        }
    }

    fn wait_for_external_scans(&self) {
        let registrations = self.external_scans();
        drop(
            self.external_scan_wake
                .wait_while(registrations, |state| {
                    !state.admissions.is_empty() || !state.registrations.is_empty()
                })
                .unwrap_or_else(|poison| poison.into_inner()),
        );
    }

    fn wait_for_external_scans_for_sources(&self, source_ids: &BTreeSet<String>) {
        let registrations = self.external_scans();
        drop(
            self.external_scan_wake
                .wait_while(registrations, |state| {
                    state
                        .admissions
                        .values()
                        .any(|source_id| source_ids.contains(source_id))
                        || state
                            .registrations
                            .values()
                            .any(|registration| source_ids.contains(&registration.source_id))
                })
                .unwrap_or_else(|poison| poison.into_inner()),
        );
    }

    fn finish_external_scan_admission(&self, admission_id: u64) {
        let mut external_scans = self.external_scans();
        external_scans.admissions.remove(&admission_id);
        drop(external_scans);
        self.external_scan_wake.notify_all();
    }

    fn begin_in_flight_work<'a>(
        &'a self,
        source_id: &str,
        expected_cancel: &Arc<AtomicBool>,
    ) -> Option<InFlightWorkGuard<'a>> {
        let control = self.control();
        let current_cancel = control.source_work_cancels.get(source_id)?;
        if control.shutdown
            || control.processing_paused()
            || !control.source_is_active(source_id)
            || !Arc::ptr_eq(current_cancel, expected_cancel)
            || expected_cancel.load(Ordering::Acquire)
        {
            return None;
        }
        let mut in_flight = self
            .in_flight_work
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *in_flight.entry(source_id.to_string()).or_default() += 1;
        drop(in_flight);
        drop(control);
        Some(InFlightWorkGuard {
            shared: self,
            source_id: source_id.to_string(),
        })
    }

    fn wait_for_in_flight_sources(&self, source_ids: &BTreeSet<String>) {
        let in_flight = self
            .in_flight_work
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        drop(
            self.in_flight_wake
                .wait_while(in_flight, |work| {
                    source_ids
                        .iter()
                        .any(|source_id| work.get(source_id).is_some_and(|count| *count > 0))
                })
                .unwrap_or_else(|poison| poison.into_inner()),
        );
    }
}

struct ControlState {
    sources: BTreeMap<String, SampleSource>,
    source_work_cancels: BTreeMap<String, Arc<AtomicBool>>,
    dirty_sources: BTreeSet<String>,
    quarantined_sources: BTreeSet<String>,
    wake_generation: u64,
    wake_reason: &'static str,
    playback_active: bool,
    foreground_active: bool,
    shutdown: bool,
    priority: PriorityContext,
}

impl ControlState {
    fn source_is_active(&self, source_id: &str) -> bool {
        self.sources.contains_key(source_id) && !self.quarantined_sources.contains(source_id)
    }

    fn notify(&mut self, reason: &'static str) {
        self.wake_generation = self.wake_generation.wrapping_add(1);
        self.wake_reason = reason;
    }

    fn mark_source_dirty(&mut self, source_id: &str, reason: &'static str) {
        if self.source_is_active(source_id) {
            self.dirty_sources.insert(source_id.to_string());
            self.notify(reason);
        }
    }

    fn mark_all_sources_dirty(&mut self, reason: &'static str) {
        self.dirty_sources.extend(
            self.sources
                .keys()
                .filter(|source_id| !self.quarantined_sources.contains(*source_id))
                .cloned(),
        );
        self.notify(reason);
    }

    fn processing_paused(&self) -> bool {
        self.playback_active || self.foreground_active
    }

    fn cancel_source_work(&mut self, source_id: &str) {
        if let Some(cancel) = self.source_work_cancels.get_mut(source_id) {
            cancel.store(true, Ordering::Release);
            if !self.quarantined_sources.contains(source_id) {
                *cancel = Arc::new(AtomicBool::new(false));
            }
        }
    }

    fn cancel_all_source_work(&mut self) {
        for cancel in self.source_work_cancels.values() {
            cancel.store(true, Ordering::Release);
        }
    }

    fn reset_source_work_tokens(&mut self) {
        self.source_work_cancels = self
            .sources
            .keys()
            .map(|source_id| {
                let cancelled = self.quarantined_sources.contains(source_id);
                (source_id.clone(), Arc::new(AtomicBool::new(cancelled)))
            })
            .collect();
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
    source_discoveries: u64,
}

#[derive(Clone, Debug)]
enum RuntimeTask {
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

enum Cancellable<T> {
    Completed(T),
    Cancelled,
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
    let mut next_safety_sweep_at = Instant::now() + SAFETY_SWEEP_INTERVAL;
    let mut scheduler = FairScheduler::default();
    let mut reset_sources = BTreeMap::<String, bool>::new();
    let mut candidates = Vec::<RuntimeCandidate>::new();
    let mut source_stats = BTreeMap::<String, SourceDiscoveryStats>::new();
    loop {
        let (sources, dirty_sources, source_work_cancels, processing_paused, generation, reason) = {
            let mut control = shared.control();
            while !control.shutdown && control.wake_generation == observed_generation {
                let wait_duration = coordinator_wait_duration(
                    next_retry_at,
                    now_epoch_seconds(),
                    next_safety_sweep_at.saturating_duration_since(Instant::now()),
                );
                let (next, _) = shared
                    .wake
                    .wait_timeout(control, wait_duration)
                    .unwrap_or_else(|poison| poison.into_inner());
                control = next;
                if control.wake_generation == observed_generation {
                    let now = now_epoch_seconds();
                    if Instant::now() >= next_safety_sweep_at {
                        control.mark_all_sources_dirty("periodic_safety_sweep");
                        next_safety_sweep_at = Instant::now() + SAFETY_SWEEP_INTERVAL;
                    } else {
                        let due_sources = source_stats
                            .iter()
                            .filter_map(|(source_id, stats)| {
                                stats
                                    .earliest_retry_at
                                    .is_some_and(|deadline| deadline <= now)
                                    .then(|| source_id.clone())
                            })
                            .collect::<Vec<_>>();
                        for source_id in due_sources {
                            control.dirty_sources.insert(source_id);
                        }
                        control.notify("retry_deadline");
                    }
                }
            }
            if control.shutdown {
                break;
            }
            let processing_paused = control.processing_paused();
            let dirty_sources = if processing_paused {
                BTreeSet::new()
            } else {
                std::mem::take(&mut control.dirty_sources)
            };
            (
                control
                    .sources
                    .iter()
                    .filter(|(source_id, _)| control.source_is_active(source_id))
                    .map(|(_, source)| source.clone())
                    .collect::<Vec<_>>(),
                dirty_sources,
                control.source_work_cancels.clone(),
                processing_paused,
                control.wake_generation,
                control.wake_reason,
            )
        };
        observed_generation = generation;
        scheduler.set_paused(processing_paused);
        let configured_source_ids = sources
            .iter()
            .map(|source| source.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        candidates.retain(|candidate| {
            configured_source_ids.contains(candidate.source.id.as_str())
                && !dirty_sources.contains(candidate.source.id.as_str())
        });
        source_stats.retain(|source_id, _| configured_source_ids.contains(source_id));
        if processing_paused {
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
                let Some(source_cancel) = source_work_cancels.get(&source_id) else {
                    shared.budgets().release(permit);
                    shared.budget_wake.notify_all();
                    continue;
                };
                let Some(in_flight_work) = shared.begin_in_flight_work(&source_id, source_cancel)
                else {
                    shared.budgets().release(permit);
                    shared.budget_wake.notify_all();
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
                drop(in_flight_work);
                shared.budgets().release(permit);
                shared.budget_wake.notify_all();
            }
        }
        reset_sources
            .retain(|source_id, _| sources.iter().any(|source| source.id.as_str() == source_id));

        let sweep_started = Instant::now();
        let sources_to_discover = sources
            .iter()
            .filter(|source| dirty_sources.contains(source.id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for source in &sources_to_discover {
            source_stats.remove(source.id.as_str());
        }
        let (mut discovered, discovered_source_stats, deferred_discoveries) =
            discover_candidates(&shared, &sources_to_discover, &source_work_cancels);
        if !deferred_discoveries.is_empty() {
            shared.control().dirty_sources.extend(deferred_discoveries);
        }
        candidates.append(&mut discovered);
        source_stats.extend(discovered_source_stats);
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
            let interrupted = control.processing_paused() || !control.dirty_sources.is_empty();
            let priority = control.priority.clone();
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
            let Some(candidate_cancel) = source_work_cancels.get(candidate.source.id.as_str())
            else {
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
            let result = execute_candidate(&candidate, candidate_cancel.as_ref());
            drop(in_flight_work);
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
            let requeue_cancelled = matches!(execution_outcome, Some(ExecutionOutcome::Cancelled))
                && {
                    let control = shared.control();
                    control.source_is_active(candidate.source.id.as_str())
                        && !control.dirty_sources.contains(candidate.source.id.as_str())
                };
            if requeue_cancelled {
                candidates.push(candidate);
                continue;
            }
            let source_id = candidate.source.id.as_str();
            if candidates
                .iter()
                .any(|queued| queued.source.id.as_str() == source_id)
            {
                continue;
            }
            let should_refresh = match (&candidate.task, execution_outcome) {
                (RuntimeTask::LegacyAnalysis { .. }, Some(ExecutionOutcome::Completed))
                | (RuntimeTask::LegacyAnalysis { .. }, Some(ExecutionOutcome::Failed))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Completed))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Retried { .. }))
                | (RuntimeTask::Readiness(_), Some(ExecutionOutcome::Failed)) => true,
                _ => false,
            };
            if !should_refresh {
                continue;
            }
            shared
                .control()
                .mark_source_dirty(source_id, "source_stage_progress");
            break;
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
    source_work_cancels: &BTreeMap<String, Arc<AtomicBool>>,
) -> (
    Vec<RuntimeCandidate>,
    BTreeMap<String, SourceDiscoveryStats>,
    BTreeSet<String>,
) {
    let now = now_epoch_seconds();
    let mut candidates = Vec::new();
    let mut source_stats = BTreeMap::new();
    let mut deferred = BTreeSet::new();
    for source in sources {
        let Some(permit) = shared
            .budgets()
            .try_acquire(source.id.as_str(), ProcessingLane::Cleanup)
        else {
            deferred.insert(source.id.as_str().to_string());
            continue;
        };
        let Some(source_cancel) = source_work_cancels.get(source.id.as_str()) else {
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            continue;
        };
        let Some(in_flight_work) = shared.begin_in_flight_work(source.id.as_str(), source_cancel)
        else {
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            continue;
        };
        {
            let mut telemetry = shared.telemetry();
            telemetry.source_discoveries = telemetry.source_discoveries.saturating_add(1);
        }
        match discover_source_candidates(source, now, source_cancel) {
            Ok(Cancellable::Completed((mut source_candidates, stats))) => {
                candidates.append(&mut source_candidates);
                source_stats.insert(source.id.as_str().to_string(), stats);
            }
            Ok(Cancellable::Cancelled) => {
                deferred.insert(source.id.as_str().to_string());
            }
            Err(error) => record_discovery_error(shared, source, &error),
        }
        drop(in_flight_work);
        shared.budgets().release(permit);
        shared.budget_wake.notify_all();
    }
    (candidates, source_stats, deferred)
}

fn discover_source_candidates(
    source: &SampleSource,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    discover_source_candidates_with_connection(source, &mut connection, now, cancel)
}

fn discover_source_candidates_with_connection(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    let source_id = source.id.as_str();
    let mut candidates = Vec::new();
    let mut stats = SourceDiscoveryStats::default();
    if connection
        .is_readonly(rusqlite::MAIN_DB)
        .map_err(|error| error.to_string())?
    {
        tracing::debug!(
            target: "wavecrate::source_processing",
            source_id,
            "Source processing is disabled for a read-only source database"
        );
        return Ok(Cancellable::Completed((candidates, stats)));
    }
    if !source_processing_schema_available(&connection)? {
        tracing::debug!(
            target: "wavecrate::source_processing",
            source_id,
            "Source processing is unavailable until the read-only source database is migrated"
        );
        return Ok(Cancellable::Completed((candidates, stats)));
    }
    if matches!(
        publish_current_readiness_targets_with_cancel(connection, source_id, now, cancel)?,
        Cancellable::Cancelled
    ) {
        return Ok(Cancellable::Cancelled);
    }
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let readiness_source_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM source_readiness_sources WHERE source_id = ?1)",
            [source_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if readiness_source_exists {
        let snapshot = match reconcile_readiness_with_cancel(&connection, source_id, now, cancel) {
            Ok(snapshot) => snapshot,
            Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
                return Ok(Cancellable::Cancelled);
            }
            Err(error) => return Err(error.to_string()),
        };
        match persist_readiness_deficits_with_cancel(connection, &snapshot.deficits, now, cancel) {
            Ok(_) => {}
            Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
                return Ok(Cancellable::Cancelled);
            }
            Err(error) => return Err(error.to_string()),
        }
        stats.readiness_queue_depth = snapshot.deficits.len();
        candidates.extend(snapshot.deficits.iter().map(|deficit| RuntimeCandidate {
            schedule: WorkCandidate::readiness(&deficit.target, deficit.enqueued_at.unwrap_or(now)),
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
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
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
    if cancelled(cancel) {
        Ok(Cancellable::Cancelled)
    } else {
        Ok(Cancellable::Completed((candidates, stats)))
    }
}

fn source_processing_schema_available(connection: &rusqlite::Connection) -> Result<bool, String> {
    for (table, required_columns) in [
        (
            "wav_files",
            &[
                "path",
                "file_identity",
                "content_hash",
                "file_size",
                "modified_ns",
                "missing",
            ][..],
        ),
        (
            "analysis_jobs",
            &[
                "id",
                "relative_path",
                "job_type",
                "created_at",
                "status",
                "readiness_managed",
            ][..],
        ),
        ("metadata", &["key", "value"][..]),
    ] {
        let pragma = format!("PRAGMA table_info({table})");
        let mut statement = connection
            .prepare(&pragma)
            .map_err(|error| error.to_string())?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| error.to_string())?
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .map_err(|error| error.to_string())?;
        if required_columns
            .iter()
            .any(|column| !columns.contains(*column))
        {
            return Ok(false);
        }
    }
    connection
        .query_row(
            "SELECT COUNT(*) = 3
             FROM sqlite_master
             WHERE type = 'table'
               AND name IN (
                   'source_readiness_sources',
                   'source_readiness_targets',
                   'source_readiness_artifacts'
               )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn fence_retired_source_readiness(source: &SampleSource) -> Result<(), String> {
    let database_path = source.db_path().map_err(|error| error.to_string())?;
    if !database_path.exists() {
        return Ok(());
    }
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    if connection
        .is_readonly(rusqlite::MAIN_DB)
        .map_err(|error| error.to_string())?
    {
        return Ok(());
    }
    let readiness_source_table_exists = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'source_readiness_sources'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| error.to_string())?;
    if !readiness_source_table_exists {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "UPDATE source_readiness_sources
             SET availability = 'disabled',
                 readiness_revision = readiness_revision + 1,
                 updated_at = ?2
             WHERE source_id = ?1 AND availability != 'disabled'",
            params![source.id.as_str(), now_epoch_seconds()],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

#[cfg(test)]
fn publish_current_readiness_targets(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
) -> Result<bool, String> {
    let cancel = AtomicBool::new(false);
    match publish_current_readiness_targets_with_cancel(connection, source_id, now, &cancel)? {
        Cancellable::Completed(changed) => Ok(changed),
        Cancellable::Cancelled => unreachable!("an uncancelled publication cannot be cancelled"),
    }
}

fn publish_current_readiness_targets_with_cancel(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<bool>, String> {
    publish_current_readiness_targets_with_cancel_and_checkpoint(
        connection,
        source_id,
        now,
        cancel,
        &mut || {},
    )
}

fn publish_current_readiness_targets_with_cancel_and_checkpoint(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
    checkpoint: &mut impl FnMut(),
) -> Result<Cancellable<bool>, String> {
    checkpoint();
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT path, file_identity, content_hash, file_size, modified_ns
                 FROM wav_files
                 WHERE missing = 0
                 ORDER BY path",
            )
            .map_err(|error| error.to_string())?;
        let mut query = statement.query([]).map_err(|error| error.to_string())?;
        let mut rows = Vec::new();
        while let Some(row) = query.next().map_err(|error| error.to_string())? {
            checkpoint();
            if cancelled(cancel) {
                return Ok(Cancellable::Cancelled);
            }
            rows.push((
                row.get::<_, String>(0).map_err(|error| error.to_string())?,
                row.get::<_, Option<String>>(1)
                    .map_err(|error| error.to_string())?,
                row.get::<_, Option<String>>(2)
                    .map_err(|error| error.to_string())?,
                row.get::<_, i64>(3).map_err(|error| error.to_string())?,
                row.get::<_, i64>(4).map_err(|error| error.to_string())?,
            ));
        }
        rows
    };
    let mut manifest = Vec::with_capacity(rows.len());
    for (path, identity, content_hash, file_size, modified_ns) in rows {
        checkpoint();
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        if !wavecrate_library::sample_sources::is_supported_audio(std::path::Path::new(&path)) {
            continue;
        }
        let Some(identity) = identity.filter(|value| !value.trim().is_empty()) else {
            mark_readiness_temporarily_unavailable(connection, source_id, now)?;
            return Ok(Cancellable::Completed(false));
        };
        let content_hash = content_hash
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("pending-{identity}-{file_size}-{modified_ns}"));
        manifest.push((path, identity, content_hash));
    }
    let source_generation = connection
        .query_row(
            "SELECT COALESCE(
                (SELECT CAST(value AS INTEGER) FROM metadata WHERE key = ?1),
                0
             )",
            [META_WAV_PATHS_REVISION],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let embedding_version = format!(
        "{}+{}",
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    );
    let mut membership = blake3::Hasher::new();
    let mut targets = Vec::with_capacity(manifest.len().saturating_mul(4).saturating_add(1));
    for (path, identity, content_hash) in &manifest {
        checkpoint();
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        membership.update(identity.as_bytes());
        membership.update(&[0]);
        membership.update(content_hash.as_bytes());
        membership.update(&[0xff]);
        for (stage, version) in [
            (ReadinessStage::IndexedIdentity, READINESS_MANIFEST_VERSION),
            (ReadinessStage::PlaybackSummary, READINESS_PLAYBACK_VERSION),
            (
                ReadinessStage::AnalysisFeatures,
                wavecrate_analysis::analysis_version(),
            ),
            (ReadinessStage::EmbeddingAspects, embedding_version.as_str()),
        ] {
            targets.push(ReadinessTarget::file(
                source_id,
                identity,
                path,
                stage,
                version,
                source_generation,
                content_hash,
            ));
        }
    }
    let membership_generation = membership.finalize().to_hex().to_string();
    targets.push(ReadinessTarget::source(
        source_id,
        ReadinessStage::SimilarityLayout,
        NATIVE_SIMILARITY_UMAP_VERSION,
        source_generation,
        membership_generation,
    ));
    let Some(target_fingerprint) = readiness_target_fingerprint_with_cancel(&targets, cancel)
    else {
        return Ok(Cancellable::Cancelled);
    };
    let current_fingerprint = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            [META_READINESS_TARGET_FINGERPRINT],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let current_source: Option<(i64, i64, String)> = connection
        .query_row(
            "SELECT source_generation, readiness_revision, availability
             FROM source_readiness_sources WHERE source_id = ?1",
            [source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if current_fingerprint.as_deref() == Some(target_fingerprint.as_str())
        && current_source
            .as_ref()
            .is_some_and(|(generation, _, availability)| {
                *generation == source_generation && availability == "active"
            })
    {
        return Ok(Cancellable::Completed(false));
    }
    let readiness_revision = current_source
        .map(|(_, revision, _)| revision.saturating_add(1))
        .unwrap_or(1);
    match replace_readiness_targets_with_cancel(
        connection,
        source_id,
        source_generation,
        readiness_revision,
        SourceAvailability::Active,
        &targets,
        now,
        cancel,
    ) {
        Ok(()) => {}
        Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
            return Ok(Cancellable::Cancelled);
        }
        Err(error) => return Err(error.to_string()),
    }
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    connection
        .execute(
            "INSERT INTO metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_READINESS_TARGET_FINGERPRINT, target_fingerprint],
        )
        .map_err(|error| error.to_string())?;
    Ok(Cancellable::Completed(true))
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Acquire)
}

fn mark_readiness_temporarily_unavailable(
    connection: &rusqlite::Connection,
    source_id: &str,
    now: i64,
) -> Result<(), String> {
    connection
        .execute(
            "UPDATE source_readiness_sources
             SET availability = 'offline',
                 readiness_revision = readiness_revision + 1,
                 updated_at = ?2
             WHERE source_id = ?1 AND availability != 'offline'",
            params![source_id, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn readiness_target_fingerprint_with_cancel(
    targets: &[ReadinessTarget],
    cancel: &AtomicBool,
) -> Option<String> {
    let mut hash = blake3::Hasher::new();
    for target in targets {
        if cancelled(cancel) {
            return None;
        }
        hash.update(target.source_id.as_bytes());
        hash.update(&[0]);
        hash.update(target.scope_id.as_bytes());
        hash.update(&[0]);
        hash.update(format!("{:?}", target.stage).as_bytes());
        hash.update(&[0]);
        hash.update(target.required_version.as_bytes());
        hash.update(&[0]);
        hash.update(target.source_generation.to_string().as_bytes());
        hash.update(&[0]);
        hash.update(target.content_generation.as_bytes());
        hash.update(&[0xff]);
    }
    Some(hash.finalize().to_hex().to_string())
}

fn execute_candidate(
    candidate: &RuntimeCandidate,
    cancel: &AtomicBool,
) -> Result<ExecutionOutcome, String> {
    let result = match &candidate.task {
        RuntimeTask::LegacyAnalysis { job_id } => {
            super::worker::run_legacy_job(&candidate.source, *job_id, 1, cancel).map(
                |(processed, failed)| {
                    if processed == 0 {
                        ExecutionOutcome::NotClaimed
                    } else if failed > 0 {
                        ExecutionOutcome::Failed
                    } else {
                        ExecutionOutcome::Completed
                    }
                },
            )
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
        |lease_cancel| run_readiness_stage(source, &mut connection, &claim, lease_cancel),
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
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, READINESS_MAX_ATTEMPTS)
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
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, READINESS_MAX_ATTEMPTS)
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
    connection: &mut rusqlite::Connection,
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
                let has_content_hash: bool = connection
                    .query_row(
                        "SELECT EXISTS(
                            SELECT 1 FROM wav_files
                            WHERE file_identity = ?1 AND path = ?2 AND missing = 0
                              AND content_hash IS NOT NULL AND content_hash != ''
                        )",
                        params![target.scope_id, relative_path],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())?;
                if !has_content_hash {
                    let database_root =
                        source.database_root().map_err(|error| error.to_string())?;
                    let db = SourceDatabase::open_for_background_job_with_database_root(
                        &source.root,
                        database_root,
                    )
                    .map_err(|error| error.to_string())?;
                    complete_pending_deep_hash_for_path(
                        &db,
                        std::path::Path::new(relative_path),
                        Some(cancel),
                    )
                    .map_err(|error| error.to_string())?;
                }
                let hash_is_current: bool = connection
                    .query_row(
                        "SELECT EXISTS(
                            SELECT 1 FROM wav_files
                            WHERE file_identity = ?1 AND path = ?2 AND missing = 0
                              AND content_hash IS NOT NULL AND content_hash != ''
                        )",
                        params![target.scope_id, relative_path],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())?;
                if hash_is_current {
                    ReadinessExecutionOutcome::Complete
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "indexed identity content hash is not durable yet",
                    )
                }
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
            let absolute_path = source.root.join(relative_path);
            if !cached_waveform_file_audition_ready_exists(&absolute_path) {
                ensure_persisted_playback_summary(absolute_path, cancel)?;
            }
            Ok(ReadinessExecutionOutcome::Complete)
        }
        ReadinessStage::AnalysisFeatures => {
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "analysis feature target has no relative path",
                ));
            };
            if target.required_version != wavecrate_analysis::analysis_version() {
                return Ok(ReadinessExecutionOutcome::Retry(
                    "feature executor version does not match target",
                ));
            }
            Ok(if analysis_features_are_current(connection, target)? {
                ReadinessExecutionOutcome::Complete
            } else {
                let produced = super::worker::run_readiness_feature_stage(
                    connection,
                    source,
                    std::path::Path::new(relative_path),
                    target.content_generation.as_str(),
                    target.required_version.as_str(),
                    cancel,
                )?;
                if produced && analysis_features_are_current(connection, target)? {
                    ReadinessExecutionOutcome::Complete
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "analysis feature source generation is not current yet",
                    )
                }
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
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "embedding target has no relative path",
                ));
            };
            Ok(if embedding_aspects_are_current(connection, target)? {
                ReadinessExecutionOutcome::Complete
            } else {
                let produced = super::worker::run_readiness_embedding_stage(
                    connection,
                    source,
                    std::path::Path::new(relative_path),
                    target.content_generation.as_str(),
                    wavecrate_analysis::analysis_version(),
                    cancel,
                )?;
                if produced && embedding_aspects_are_current(connection, target)? {
                    ReadinessExecutionOutcome::Complete
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "embedding feature prerequisite is not durable yet",
                    )
                }
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

fn analysis_features_are_current(
    connection: &rusqlite::Connection,
    target: &ReadinessTarget,
) -> Result<bool, String> {
    let Some(sample_id) = readiness_sample_id(target) else {
        return Ok(false);
    };
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM analysis_cache_features
                WHERE content_hash = ?1 AND analysis_version = ?2
            ) AND EXISTS(
                SELECT 1
                FROM samples AS sample
                JOIN features AS feature ON feature.sample_id = sample.sample_id
                WHERE sample.sample_id = ?3
                  AND sample.content_hash = ?1
                  AND sample.analysis_version = ?2
                  AND feature.feat_version = ?4
            )",
            params![
                target.content_generation,
                target.required_version,
                sample_id,
                wavecrate_analysis::vector::FEATURE_VERSION_V1,
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn embedding_aspects_are_current(
    connection: &rusqlite::Connection,
    target: &ReadinessTarget,
) -> Result<bool, String> {
    let Some(sample_id) = readiness_sample_id(target) else {
        return Ok(false);
    };
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM samples AS sample
                JOIN embeddings AS embedding
                  ON embedding.sample_id = sample.sample_id
                JOIN analysis_cache_embeddings AS cached_embedding
                  ON cached_embedding.content_hash = sample.content_hash
                 AND cached_embedding.analysis_version = ?2
                 AND cached_embedding.model_id = ?3
                JOIN similarity_aspect_descriptors AS aspects
                  ON aspects.sample_id = sample.sample_id
                JOIN analysis_cache_aspect_descriptors AS cached_aspects
                  ON cached_aspects.content_hash = sample.content_hash
                 AND cached_aspects.analysis_version = ?2
                 AND cached_aspects.model_id = ?4
                WHERE sample.sample_id = ?5
                  AND sample.content_hash = ?1
                  AND embedding.model_id = cached_embedding.model_id
                  AND embedding.dim = cached_embedding.dim
                  AND embedding.dtype = cached_embedding.dtype
                  AND embedding.l2_normed = cached_embedding.l2_normed
                  AND embedding.vec = cached_embedding.vec
                  AND embedding.dim = ?6
                  AND embedding.dtype = ?7
                  AND embedding.l2_normed = 1
                  AND aspects.model_id = cached_aspects.model_id
                  AND aspects.dim = cached_aspects.dim
                  AND aspects.dtype = cached_aspects.dtype
                  AND aspects.l2_normed = cached_aspects.l2_normed
                  AND aspects.valid_mask = cached_aspects.valid_mask
                  AND aspects.vec = cached_aspects.vec
                  AND aspects.dim = ?8
                  AND aspects.dtype = ?9
                  AND aspects.l2_normed = 1
            )",
            params![
                target.content_generation,
                wavecrate_analysis::analysis_version(),
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                sample_id,
                wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn readiness_sample_id(target: &ReadinessTarget) -> Option<String> {
    target
        .relative_path
        .as_deref()
        .map(|relative_path| format!("{}::{}", target.source_id, relative_path.replace('\\', "/")))
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

fn source_maps_match(
    current: &BTreeMap<String, SampleSource>,
    replacement: &BTreeMap<String, SampleSource>,
) -> bool {
    current.len() == replacement.len()
        && current.iter().all(|(source_id, source)| {
            replacement
                .get(source_id)
                .is_some_and(|other| source_descriptors_match(source, other))
        })
}

fn source_descriptors_match(left: &SampleSource, right: &SampleSource) -> bool {
    left.id == right.id
        && left.root == right.root
        && left.role == right.role
        && left.metadata_storage == right.metadata_storage
        && left.primary_import_folder == right.primary_import_folder
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

fn coordinator_wait_duration(
    next_retry_at: Option<i64>,
    now: i64,
    safety_wait: Duration,
) -> Duration {
    let retry_wait = next_retry_at.map_or(safety_wait, |deadline| {
        Duration::from_secs(deadline.saturating_sub(now).max(0) as u64)
    });
    safety_wait.min(retry_wait)
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
        readiness::{
            ReadinessEligibility, SourceAvailability, persist_readiness_deficits,
            reconcile_readiness, replace_readiness_targets,
        },
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
        wait_until(Duration::from_secs(10), || source_is_hashed(&source));
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
    }

    #[test]
    fn removing_a_source_cancels_its_unstarted_backlog() {
        let (_directory, source) = unhashed_source("removed");
        let mut supervisor =
            SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);
        supervisor
            .replace_sources(Vec::new())
            .expect("remove configured sources");
        supervisor.set_playback_active(false);

        thread::sleep(Duration::from_millis(150));
        assert!(!source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn failed_retirement_fence_quarantines_source_until_retry_succeeds() {
        let (_directory, source) = unhashed_source("retirement-fence-failure");
        let database_path = source.db_path().expect("source database path");
        std::fs::remove_file(&database_path).expect("remove source database");
        std::fs::create_dir(&database_path).expect("replace database with invalid directory");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");

        assert!(
            supervisor.replace_sources(Vec::new()).is_err(),
            "invalid database path must fail persistent retirement fencing"
        );
        supervisor.wake_source(source.id.as_str(), "late_watcher_event");
        supervisor.set_playback_active(true);
        supervisor.set_playback_active(false);

        let control = supervisor.shared.control();
        assert!(control.quarantined_sources.contains(source.id.as_str()));
        assert!(!control.dirty_sources.contains(source.id.as_str()));
        assert!(control.source_work_cancels[source.id.as_str()].load(Ordering::Acquire));
        drop(control);
        assert!(
            supervisor
                .budget_handle()
                .acquire_scan(source.id.as_str())
                .is_none(),
            "quarantined retirement must reject late external scans"
        );

        std::fs::remove_dir(&database_path).expect("repair invalid database path");
        supervisor
            .replace_sources(Vec::new())
            .expect("retry retirement fence");
        let control = supervisor.shared.control();
        assert!(!control.sources.contains_key(source.id.as_str()));
        assert!(control.quarantined_sources.is_empty());
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn source_wakes_cancel_only_that_sources_in_flight_generation() {
        let (_first_directory, first) = unhashed_source("first");
        let (_second_directory, second) = unhashed_source("second");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![first.clone(), second.clone()])
            .expect("configure sources");
        let (first_generation, second_generation) = {
            let control = supervisor.shared.control();
            (
                Arc::clone(&control.source_work_cancels[first.id.as_str()]),
                Arc::clone(&control.source_work_cancels[second.id.as_str()]),
            )
        };
        let first_scan = supervisor
            .budget_handle()
            .acquire_scan(first.id.as_str())
            .expect("acquire first-source scan permit");
        let first_scan_generation = first_scan.cancel_token();

        supervisor.wake_source(first.id.as_str(), "test_source_wake");

        assert!(first_generation.load(Ordering::Acquire));
        assert!(first_scan_generation.load(Ordering::Acquire));
        assert!(!second_generation.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert!(!control.source_work_cancels[first.id.as_str()].load(Ordering::Acquire));
        assert!(Arc::ptr_eq(
            &second_generation,
            &control.source_work_cancels[second.id.as_str()]
        ));
        drop(control);
        drop(first_scan);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn external_scan_release_invalidates_retained_source_generation() {
        let (_directory, source) = unhashed_source("external-commit-generation");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let retained_generation = {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };
        let permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("admit external source work");

        drop(permit);

        assert!(retained_generation.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert!(control.dirty_sources.contains(source.id.as_str()));
        assert!(!control.source_work_cancels[source.id.as_str()].load(Ordering::Acquire));
        assert!(!Arc::ptr_eq(
            &retained_generation,
            &control.source_work_cancels[source.id.as_str()]
        ));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn scan_registration_only_adds_absent_matching_sources() {
        let (_directory, source) = unhashed_source("scan-registration");
        let mut supervisor = SourceProcessingSupervisor::dormant();

        supervisor
            .register_source_for_scan(source.clone())
            .expect("register source before first scan");
        supervisor
            .register_source_for_scan(source.clone())
            .expect("matching registration is idempotent");
        let permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("newly registered source admits scan work");

        let replacement_directory = tempfile::tempdir().expect("replacement source root");
        let replacement = SampleSource::new_with_id(
            source.id.clone(),
            replacement_directory.path().to_path_buf(),
        );
        assert!(
            supervisor.register_source_for_scan(replacement).is_err(),
            "scan registration must not replace an authoritative descriptor"
        );
        let control = supervisor.shared.control();
        assert!(source_descriptors_match(
            &control.sources[source.id.as_str()],
            &source
        ));
        drop(control);
        drop(permit);

        let replacement = supervisor
            .shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        assert_eq!(
            supervisor
                .register_source_for_scan(source.clone())
                .expect_err("scan registration must not wait for source replacement"),
            "Configured sources are currently being replaced"
        );
        drop(replacement);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn identical_source_refresh_is_a_noop_and_descriptor_changes_are_source_local() {
        let (_first_directory, first) = unhashed_source("refresh-first");
        let (_second_directory, second) = unhashed_source("refresh-second");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![first.clone(), second.clone()])
            .expect("configure sources");
        let (first_generation, second_generation, wake_generation) = {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            control
                .priority
                .immediate_paths
                .insert((first.id.to_string(), "first.wav".to_string()));
            control
                .priority
                .immediate_paths
                .insert((second.id.to_string(), "second.wav".to_string()));
            control.priority.selected_source = Some(second.id.to_string());
            (
                Arc::clone(&control.source_work_cancels[first.id.as_str()]),
                Arc::clone(&control.source_work_cancels[second.id.as_str()]),
                control.wake_generation,
            )
        };

        supervisor
            .replace_sources(vec![first.clone(), second.clone()])
            .expect("refresh identical sources");

        {
            let control = supervisor.shared.control();
            assert_eq!(control.wake_generation, wake_generation);
            assert!(control.dirty_sources.is_empty());
            assert!(Arc::ptr_eq(
                &first_generation,
                &control.source_work_cancels[first.id.as_str()]
            ));
            assert!(Arc::ptr_eq(
                &second_generation,
                &control.source_work_cancels[second.id.as_str()]
            ));
        }

        let replacement_directory = tempfile::tempdir().expect("replacement source root");
        let replacement =
            SampleSource::new_with_id(first.id.clone(), replacement_directory.path().to_path_buf());
        supervisor
            .replace_sources(vec![replacement, second.clone()])
            .expect("replace changed source");

        assert!(first_generation.load(Ordering::Acquire));
        assert!(!second_generation.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert_eq!(
            control.dirty_sources,
            BTreeSet::from([first.id.to_string()])
        );
        assert!(Arc::ptr_eq(
            &second_generation,
            &control.source_work_cancels[second.id.as_str()]
        ));
        assert_eq!(
            control.priority.immediate_paths,
            BTreeSet::from([(second.id.to_string(), "second.wav".to_string())])
        );
        assert_eq!(
            control.priority.selected_source.as_deref(),
            Some(second.id.as_str())
        );
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn priority_only_wakes_reuse_candidates_without_source_rediscovery() {
        let directory = tempfile::tempdir().expect("priority source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("priority-cache"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create priority source database");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(5), || {
            let telemetry = supervisor.shared.telemetry();
            telemetry.source_discoveries >= 2 && telemetry.queue_depth == 0
        });
        let discoveries_before = supervisor.shared.telemetry().source_discoveries;

        for index in 0..128 {
            supervisor.prioritize_path(
                source.id.as_str(),
                format!("visible/sample-{index}.wav").as_str(),
                true,
            );
        }
        thread::sleep(Duration::from_millis(150));

        assert_eq!(
            supervisor.shared.telemetry().source_discoveries,
            discoveries_before,
            "priority-only wakes must reschedule the retained batch without database discovery"
        );
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn playback_and_foreground_resumes_reuse_the_retained_source_snapshot() {
        let directory = tempfile::tempdir().expect("resume source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("resume-cache"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create resume source database");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source]);
        wait_until(Duration::from_secs(5), || {
            let telemetry = supervisor.shared.telemetry();
            telemetry.source_discoveries >= 2 && telemetry.queue_depth == 0
        });
        let discoveries_before = supervisor.shared.telemetry().source_discoveries;

        for _ in 0..64 {
            supervisor.set_playback_active(true);
            supervisor.set_playback_active(false);
            supervisor.set_foreground_activity(true);
            supervisor.set_foreground_activity(false);
        }
        thread::sleep(Duration::from_millis(150));

        assert_eq!(
            supervisor.shared.telemetry().source_discoveries,
            discoveries_before,
            "resume must reuse retained candidates instead of restarting manifest discovery"
        );
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn embedding_readiness_rejects_rows_from_the_previous_content_generation() {
        let (_directory, source) = ready_analysis_source("embedding-generation");
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open embedding database");
        let sample_id = format!("{}::ready.wav", source.id);
        let embedding_dim = wavecrate_analysis::similarity::SIMILARITY_DIM as i64;
        let aspect_dim = wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64;
        let embedding_a = vec![1_u8, 2, 3, 4];
        let embedding_b = vec![5_u8, 6, 7, 8];
        let aspects_a = vec![9_u8, 10, 11, 12];
        let aspects_b = vec![13_u8, 14, 15, 16];
        connection
            .execute(
                "INSERT INTO samples (sample_id, content_hash, size, mtime_ns)
                 VALUES (?1, 'content-a', 1, 1)",
                [&sample_id],
            )
            .expect("insert sample");
        for (content_hash, embedding, aspects) in [
            ("content-a", embedding_a.as_slice(), aspects_a.as_slice()),
            ("content-b", embedding_b.as_slice(), aspects_b.as_slice()),
        ] {
            connection
                .execute(
                    "INSERT INTO analysis_cache_embeddings (
                        content_hash, analysis_version, model_id, dim, dtype,
                        l2_normed, vec, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 1)",
                    params![
                        content_hash,
                        wavecrate_analysis::analysis_version(),
                        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                        embedding_dim,
                        wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                        embedding,
                    ],
                )
                .expect("insert cached embedding");
            connection
                .execute(
                    "INSERT INTO analysis_cache_aspect_descriptors (
                        content_hash, analysis_version, model_id, dim, dtype,
                        l2_normed, valid_mask, vec, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 7, ?6, 1)",
                    params![
                        content_hash,
                        wavecrate_analysis::analysis_version(),
                        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                        aspect_dim,
                        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                        aspects,
                    ],
                )
                .expect("insert cached aspects");
        }
        connection
            .execute(
                "INSERT INTO embeddings (
                    sample_id, model_id, dim, dtype, l2_normed, vec, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 1, ?5, 1)",
                params![
                    sample_id,
                    wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                    embedding_dim,
                    wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                    embedding_a,
                ],
            )
            .expect("insert materialized embedding");
        connection
            .execute(
                "INSERT INTO similarity_aspect_descriptors (
                    sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 1, 7, ?5, 1)",
                params![
                    sample_id,
                    wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                    aspect_dim,
                    wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                    aspects_a,
                ],
            )
            .expect("insert materialized aspects");
        connection
            .execute(
                "UPDATE samples SET content_hash = 'content-b' WHERE sample_id = ?1",
                [&sample_id],
            )
            .expect("advance sample content");
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "identity-1",
            "ready.wav",
            ReadinessStage::EmbeddingAspects,
            "embedding-v1",
            1,
            "content-b",
        );

        assert!(!embedding_aspects_are_current(&connection, &target).unwrap());

        connection
            .execute(
                "UPDATE embeddings SET vec = ?2 WHERE sample_id = ?1",
                params![sample_id, embedding_b],
            )
            .expect("materialize current embedding");
        connection
            .execute(
                "UPDATE similarity_aspect_descriptors SET vec = ?2 WHERE sample_id = ?1",
                params![sample_id, aspects_b],
            )
            .expect("materialize current aspects");
        assert!(embedding_aspects_are_current(&connection, &target).unwrap());
    }

    #[test]
    fn source_removal_joins_in_flight_publication_before_disabling_readiness() {
        let (_directory, source) = unhashed_source("retired-fence");
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
                 SET file_identity = 'retired-identity', content_hash = 'retired-content'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("assign readiness identity");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 1)
            .expect("publish readiness targets");
        drop(connection);

        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let generation = {
            let control = supervisor.shared.control();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };
        let in_flight = supervisor
            .shared
            .begin_in_flight_work(source.id.as_str(), &generation)
            .expect("register in-flight source publication");
        let (removed, removal_finished) = std::sync::mpsc::channel();
        thread::scope(|scope| {
            scope.spawn(|| {
                supervisor
                    .replace_sources(Vec::new())
                    .expect("remove configured source");
                removed.send(()).expect("report source removal");
            });
            assert!(
                removal_finished
                    .recv_timeout(Duration::from_millis(50))
                    .is_err(),
                "source removal must join active publication before returning"
            );
            drop(in_flight);
            removal_finished
                .recv_timeout(Duration::from_secs(2))
                .expect("source removal completes after publication joins");
        });

        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen readiness database");
        let availability: String = connection
            .query_row(
                "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read retired source readiness");
        assert_eq!(availability, "disabled");
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    #[ignore = "representative 10k-file source discovery profile"]
    fn profile_large_source_discovery_baseline() {
        const FILE_COUNT: usize = 10_000;
        let directory = tempfile::tempdir().expect("large profile source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("large-discovery-profile"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create profile source database");
        let database_root = source.database_root().expect("profile database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open profile database");
        let transaction = connection.transaction().expect("start profile seed");
        {
            let mut insert = transaction
                .prepare(
                    "INSERT INTO wav_files (
                        path, file_size, modified_ns, file_identity, content_hash, missing,
                        extension
                     ) VALUES (?1, 1024, 1, ?2, ?3, 0, 'wav')",
                )
                .expect("prepare profile insert");
            for index in 0..FILE_COUNT {
                insert
                    .execute(params![
                        format!("profile/sample-{index:05}.wav"),
                        format!("identity-{index:05}"),
                        format!("content-{index:05}"),
                    ])
                    .expect("insert profile row");
            }
        }
        transaction.commit().expect("commit profile seed");
        drop(connection);

        let started_at = Instant::now();
        let cancel = AtomicBool::new(false);
        let Cancellable::Completed((candidates, stats)) =
            discover_source_candidates(&source, 100, &cancel).expect("discover large source")
        else {
            panic!("large source discovery unexpectedly cancelled");
        };
        let elapsed = started_at.elapsed();

        assert_eq!(candidates.len(), FILE_COUNT * 4 + 1);
        assert_eq!(stats.readiness_queue_depth, FILE_COUNT * 4 + 1);
        eprintln!(
            "large_source_discovery file_count={FILE_COUNT} candidate_count={} elapsed_ms={:.3}",
            candidates.len(),
            elapsed.as_secs_f64() * 1_000.0,
        );
    }

    #[test]
    fn large_source_discovery_cancels_mid_manifest_and_resumes_cleanly() {
        const FILE_COUNT: usize = 512;
        let directory = tempfile::tempdir().expect("large cancellation source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("large-discovery-cancel"),
            directory.path().to_path_buf(),
        );
        source
            .open_db()
            .expect("create cancellation source database");
        let database_root = source.database_root().expect("cancellation database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open cancellation database");
        let transaction = connection.transaction().expect("start cancellation seed");
        {
            let mut insert = transaction
                .prepare(
                    "INSERT INTO wav_files (
                        path, file_size, modified_ns, file_identity, content_hash, missing,
                        extension
                     ) VALUES (?1, 1024, 1, ?2, ?3, 0, 'wav')",
                )
                .expect("prepare cancellation seed");
            for index in 0..FILE_COUNT {
                insert
                    .execute(params![
                        format!("cancel/sample-{index:05}.wav"),
                        format!("cancel-identity-{index:05}"),
                        format!("cancel-content-{index:05}"),
                    ])
                    .expect("insert cancellation row");
            }
        }
        transaction.commit().expect("commit cancellation seed");

        let cancel = AtomicBool::new(false);
        let mut checkpoints = 0_usize;
        let started_at = Instant::now();
        let cancelled_outcome = publish_current_readiness_targets_with_cancel_and_checkpoint(
            &mut connection,
            source.id.as_str(),
            100,
            &cancel,
            &mut || {
                checkpoints += 1;
                if checkpoints == 128 {
                    cancel.store(true, Ordering::Release);
                }
            },
        )
        .expect("cancel manifest discovery");

        assert!(matches!(cancelled_outcome, Cancellable::Cancelled));
        assert!(started_at.elapsed() < Duration::from_secs(1));
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM source_readiness_targets WHERE source_id = ?1",
                    [source.id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count cancelled readiness targets"),
            0,
            "cancelled discovery must not publish a partial desired state"
        );

        cancel.store(false, Ordering::Release);
        assert!(
            publish_current_readiness_targets_with_cancel(
                &mut connection,
                source.id.as_str(),
                101,
                &cancel,
            )
            .is_ok_and(|outcome| matches!(outcome, Cancellable::Completed(true)))
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM source_readiness_targets WHERE source_id = ?1",
                    [source.id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count resumed readiness targets"),
            i64::try_from(FILE_COUNT * 4 + 1).expect("target count fits i64")
        );
    }

    #[test]
    fn shutdown_waits_for_external_scan_admissions_and_rejects_late_permits() {
        let (_directory, source) = unhashed_source("admission-race");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let handle = supervisor.budget_handle();
        let first = handle
            .acquire_scan(source.id.as_str())
            .expect("reserve the only scan lane");
        let first_cancel = first.cancel_token();
        let waiting_handle = handle.clone();
        let source_id = source.id.to_string();
        let waiting = thread::spawn(move || waiting_handle.acquire_scan(&source_id).is_none());
        wait_until(Duration::from_secs(2), || {
            supervisor.shared.external_scans().admissions.len() == 1
        });

        let shutdown = thread::spawn(move || supervisor.shutdown());
        wait_until(Duration::from_secs(2), || {
            first_cancel.load(Ordering::Acquire)
        });
        drop(first);

        assert!(waiting.join().expect("join waiting admission"));
        let report = shutdown.join().expect("join supervisor shutdown");
        assert_eq!(report["joined"], true);
        assert_eq!(report["external_scans_joined"], true);
    }

    #[test]
    fn foreground_activity_cancels_in_flight_work_without_reviving_old_generations() {
        let (_directory, source) = unhashed_source("foreground");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let source_generation = {
            let control = supervisor.shared.control();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };
        let scan_permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("acquire external scan permit");
        let scan_generation = scan_permit.cancel_token();

        supervisor.set_foreground_activity(true);

        assert!(source_generation.load(Ordering::Acquire));
        assert!(scan_generation.load(Ordering::Acquire));
        assert!(supervisor.shared.control().processing_paused());
        drop(scan_permit);

        supervisor.set_foreground_activity(false);

        assert!(
            source_generation.load(Ordering::Acquire),
            "resuming must not clear a token held by an interrupted worker"
        );
        let control = supervisor.shared.control();
        assert!(!control.processing_paused());
        assert!(!control.source_work_cancels[source.id.as_str()].load(Ordering::Acquire));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn read_only_discovery_does_not_publish_or_mutate_work() {
        let (_directory, source) = ready_analysis_source("read-only-discovery");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::UiRead,
        )
        .expect("open read-only source database");
        assert!(connection.is_readonly(rusqlite::MAIN_DB).unwrap());
        let counts_before = discovery_durable_counts(&connection);

        let cancel = AtomicBool::new(false);
        let Cancellable::Completed((candidates, stats)) =
            discover_source_candidates_with_connection(&source, &mut connection, 100, &cancel)
                .expect("skip read-only source processing")
        else {
            panic!("read-only discovery unexpectedly cancelled");
        };

        assert!(candidates.is_empty());
        assert_eq!(stats.readiness_queue_depth, 0);
        assert_eq!(discovery_durable_counts(&connection), counts_before);
    }

    #[test]
    fn production_supervisor_publishes_claims_and_completes_readiness_without_manual_seed() {
        let (_directory, source) = ready_analysis_source("readiness");

        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(10), || {
            let database_root = source.database_root().expect("database root");
            let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            ) else {
                return false;
            };
            connection
                .query_row(
                    "SELECT COUNT(*) = 4
                        FROM source_readiness_sources AS source
                        JOIN source_readiness_targets AS target
                          ON target.source_id = source.source_id
                        JOIN source_readiness_artifacts AS artifact
                          ON artifact.source_id = target.source_id
                         AND artifact.scope_kind = target.scope_kind
                         AND artifact.scope_id = target.scope_id
                         AND artifact.stage = target.stage
                        WHERE source.source_id = ?1
                          AND source.availability = 'active'
                          AND target.scope_id = 'identity-1'
                          AND target.stage IN (
                              'indexed_identity', 'playback_summary',
                              'analysis_features', 'embedding_aspects'
                          )
                          AND artifact.artifact_version = target.required_version
                          AND artifact.content_generation = target.content_generation
                          AND artifact.source_generation = target.source_generation",
                    params![source.id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap_or(false)
        });
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
        assert!(report["claimed"].as_u64().unwrap_or_default() >= 1);
        assert!(report["completed"].as_u64().unwrap_or_default() >= 4);
        assert!(cached_waveform_file_audition_ready_exists(
            &source.root.join("ready.wav")
        ));
    }

    #[test]
    fn unavailable_hash_path_backs_off_without_starving_later_files() {
        let directory = tempfile::tempdir().expect("temporary hash source");
        let good_path = directory.path().join("z-good.wav");
        std::fs::write(&good_path, [7_u8; 64]).expect("write hashable sample");
        let source = SampleSource::new_with_id(
            SourceId::from_string("hash-fairness"),
            directory.path().to_path_buf(),
        );
        let db = source.open_db().expect("open hash source database");
        db.upsert_file(Path::new("a-unavailable.wav"), 64, 1)
            .expect("insert unavailable hash row");
        db.upsert_file(Path::new("z-good.wav"), 64, 1)
            .expect("insert good hash row");
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open hash database");
        connection
            .execute_batch(
                "UPDATE wav_files SET file_identity = 'identity-bad'
                 WHERE path = 'a-unavailable.wav';
                 UPDATE wav_files SET file_identity = 'identity-good'
                 WHERE path = 'z-good.wav';",
            )
            .expect("assign hash identities");
        drop(connection);

        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(10), || {
            source
                .open_db()
                .expect("open hash source")
                .entry_for_path(Path::new("z-good.wav"))
                .expect("read good hash row")
                .and_then(|entry| entry.content_hash)
                .is_some()
        });
        assert_eq!(supervisor.shutdown()["joined"], true);

        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen hash database");
        let failure: (String, Option<String>, Option<i64>, i64) = connection
            .query_row(
                "SELECT status, failure_kind, retry_at, attempts
                 FROM analysis_jobs
                 WHERE readiness_managed = 1
                   AND readiness_stage = 'indexed_identity'
                   AND relative_path = 'a-unavailable.wav'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read durable hash failure");
        assert_eq!(failure.0, "failed");
        assert_eq!(failure.1.as_deref(), Some("retryable"));
        assert!(failure.2.is_some());
        assert_eq!(failure.3, 1);
    }

    #[test]
    fn legacy_source_schema_is_not_eligible_for_automatic_processing() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                 );
                 CREATE TABLE analysis_jobs (
                    id INTEGER PRIMARY KEY,
                    relative_path TEXT NOT NULL,
                    job_type TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    status TEXT NOT NULL
                 );
                 CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .unwrap();

        assert!(!source_processing_schema_available(&connection).unwrap());
    }

    #[test]
    fn real_hash_execution_waits_for_shared_scan_database_budget() {
        let (_directory, source) = unhashed_source("shared-budget");
        let shared = Arc::new(Shared::new(vec![source.clone()]));
        let permit = SourceProcessingBudgetHandle {
            shared: Arc::clone(&shared),
        }
        .acquire_scan(source.id.as_str())
        .expect("acquire external scan budget");
        let coordinator_shared = Arc::clone(&shared);
        let coordinator = thread::Builder::new()
            .name(String::from("wavecrate-source-supervisor-test"))
            .spawn(move || run_coordinator(coordinator_shared))
            .expect("spawn source processing supervisor");
        let mut supervisor = SourceProcessingSupervisor {
            shared,
            coordinator: Some(coordinator),
        };
        thread::sleep(Duration::from_millis(150));
        assert!(!source_is_hashed(&source));

        drop(permit);
        wait_until(Duration::from_secs(3), || source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn external_scan_tokens_cancel_for_playback_removal_and_shutdown() {
        let (_directory, source) = unhashed_source("external-cancel");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");

        let playback_permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("acquire playback permit");
        let playback_cancel = playback_permit.cancel_token();
        assert!(!playback_cancel.load(Ordering::Acquire));
        supervisor.set_playback_active(true);
        assert!(playback_cancel.load(Ordering::Acquire));
        drop(playback_permit);

        supervisor.set_playback_active(false);
        let removal_permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("acquire removal permit");
        let removal_cancel = removal_permit.cancel_token();
        let removal_cancel_for_releaser = Arc::clone(&removal_cancel);
        let releaser = thread::spawn(move || {
            while !removal_cancel_for_releaser.load(Ordering::Acquire) {
                thread::yield_now();
            }
            drop(removal_permit);
        });
        supervisor
            .replace_sources(Vec::new())
            .expect("remove configured source");
        releaser.join().expect("join removal scan releaser");
        assert!(removal_cancel.load(Ordering::Acquire));

        supervisor
            .replace_sources(vec![source.clone()])
            .expect("restore configured source");
        let shutdown_permit = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("acquire shutdown permit");
        let shutdown_cancel = shutdown_permit.cancel_token();
        let release_cancel = Arc::clone(&shutdown_cancel);
        let releaser = thread::spawn(move || {
            while !release_cancel.load(Ordering::Acquire) {
                thread::yield_now();
            }
            drop(shutdown_permit);
        });
        let wake_generation = supervisor.shared.control().wake_generation;
        let report = supervisor.shutdown();
        releaser.join().expect("join external scan releaser");
        assert_eq!(report["joined"], true);
        assert_eq!(report["external_scans_joined"], true);
        assert!(shutdown_cancel.load(Ordering::Acquire));
        assert!(supervisor.shared.control().wake_generation > wake_generation);
    }

    #[test]
    fn readiness_candidates_preserve_durable_queue_creation_time() {
        let (_directory, source) = unhashed_source("queue-age");
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
                 SET file_identity = 'queue-identity', content_hash = 'queue-content'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("assign queue identity");
        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
                .expect("publish current targets")
        );
        let snapshot =
            reconcile_readiness(&connection, source.id.as_str(), 100).expect("reconcile targets");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, 100)
            .expect("persist deficits");
        drop(connection);

        let cancel = AtomicBool::new(false);
        let Cancellable::Completed((candidates, _)) =
            discover_source_candidates(&source, 250, &cancel).expect("rediscover work")
        else {
            panic!("source rediscovery unexpectedly cancelled");
        };
        let readiness = candidates
            .iter()
            .filter(|candidate| matches!(candidate.task, RuntimeTask::Readiness(_)))
            .collect::<Vec<_>>();
        assert!(!readiness.is_empty());
        assert!(
            readiness
                .iter()
                .all(|candidate| candidate.schedule.enqueued_at == 100)
        );
        assert_eq!(oldest_job_age_seconds(&candidates, 250), 150);
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
            coordinator_wait_duration(Some(105), 100, SAFETY_SWEEP_INTERVAL),
            Duration::from_secs(5)
        );
        assert_eq!(
            coordinator_wait_duration(Some(100), 100, SAFETY_SWEEP_INTERVAL),
            Duration::ZERO
        );
        assert_eq!(
            coordinator_wait_duration(Some(200), 100, SAFETY_SWEEP_INTERVAL),
            SAFETY_SWEEP_INTERVAL
        );
        assert_eq!(
            coordinator_wait_duration(None, 100, SAFETY_SWEEP_INTERVAL),
            SAFETY_SWEEP_INTERVAL
        );
        assert_eq!(
            coordinator_wait_duration(None, 100, Duration::from_secs(3)),
            Duration::from_secs(3),
            "priority wakes must preserve the remaining absolute safety-sweep deadline"
        );
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
        let mut batch = db.write_batch().expect("open identity batch");
        batch
            .set_file_identity(Path::new("pending.wav"), Some(&format!("identity-{id}")))
            .expect("assign pending identity");
        batch.commit().expect("commit pending identity");
        (directory, source)
    }

    fn ready_analysis_source(id: &str) -> (tempfile::TempDir, SampleSource) {
        let directory = tempfile::tempdir().expect("temporary readiness source");
        let path = directory.path().join("ready.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("create readiness wav");
        for index in 0..4_096 {
            let phase = index as f32 / 32.0;
            writer
                .write_sample((phase.sin() * i16::MAX as f32 * 0.25) as i16)
                .expect("write readiness sample");
        }
        writer.finalize().expect("finalize readiness wav");
        let size = path.metadata().expect("read readiness metadata").len();
        let content_hash = blake3::hash(&std::fs::read(&path).expect("read readiness wav"))
            .to_hex()
            .to_string();
        let source =
            SampleSource::new_with_id(SourceId::from_string(id), directory.path().to_path_buf());
        let db = source.open_db().expect("open readiness source database");
        db.upsert_file(Path::new("ready.wav"), size, 1)
            .expect("insert readiness wav row");
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'identity-1', content_hash = ?1
                 WHERE path = 'ready.wav'",
                [&content_hash],
            )
            .expect("assign readiness identity");
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

    fn discovery_durable_counts(connection: &rusqlite::Connection) -> (i64, i64, i64) {
        connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM source_readiness_targets),
                    (SELECT COUNT(*) FROM analysis_jobs),
                    (SELECT COUNT(*) FROM metadata WHERE key = ?1)",
                [META_READINESS_TARGET_FINGERPRINT],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read durable discovery counts")
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
