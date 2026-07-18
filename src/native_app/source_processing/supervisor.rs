#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::Sender,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole, SourceMetadataStorage,
    db::{META_LAST_MANIFEST_AUDIT_AT, META_WAV_PATHS_REVISION},
    readiness::{
        ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessEligibility,
        ReadinessFailureClassification, ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome,
        ReadinessRetryPolicy, ReadinessStage, ReadinessTarget, ReadinessWorkMutationOutcome,
        SourceAvailability, cancel_readiness_work, claim_readiness_target, complete_readiness_work,
        complete_readiness_work_with_artifact_ref, fail_readiness_work,
        invalidate_readiness_artifact, persist_readiness_deficits_with_cancel,
        readiness_work_stats, reconcile_readiness_with_cancel, release_readiness_work,
        renew_readiness_lease, replace_readiness_targets_with_cancel,
    },
    scanner::{
        ScanError, audit_source_and_record_with_progress, complete_pending_deep_hash_for_path,
        sync_paths_with_progress,
    },
};

use super::scheduler::{
    BudgetTracker, FairScheduler, PriorityContext, ProcessingBudgets, ProcessingLane, WorkCandidate,
};
use crate::native_app::app::{GuiMessage, SourceProcessingProgress};
use crate::native_app::sample_library::similarity_artifacts::{
    SimilarityPublicationFence, finalize_similarity_artifacts_if_ready,
    native_similarity_artifact_version, reset_interrupted_readiness_jobs,
};
use crate::native_app::waveform::{
    ensure_persisted_playback_summary, invalidate_persisted_waveform_cache_path,
    invalidate_persisted_waveform_cache_ref, persisted_waveform_cache_ref_is_current,
    remap_persisted_waveform_cache_ref_after_move,
};

const SAFETY_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const PROGRESS_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const SIMILARITY_SCORE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const MANIFEST_AUDIT_INTERVAL_SECONDS: i64 = 24 * 60 * 60;
const MANIFEST_AUDIT_HASH_BATCH: usize = 8;
const MAX_VISIBLE_PRIORITY_PATHS: usize = 128;
const READINESS_LEASE_SECONDS: i64 = 5 * 60;
const READINESS_MAX_ATTEMPTS: u32 = 8;
const READINESS_MANIFEST_VERSION: &str = "source_manifest_v1";
const READINESS_PLAYBACK_VERSION: &str = "waveform_cache_v5_owned";
const META_READINESS_TARGET_FINGERPRINT: &str = "readiness_target_fingerprint_v1";
const SOURCE_RETIREMENT_RETRY_SECONDS: i64 = 5;
const ORPHAN_CACHE_MIN_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const ORPHAN_CACHE_MAX_SCANNED: usize = 4_096;
const ORPHAN_CACHE_MAX_REMOVED: usize = 32;
const RETAINED_SOURCE_MAX_SCANNED: usize = 1_024;
static ORPHAN_CACHE_SCAN_CURSOR: AtomicUsize = AtomicUsize::new(0);

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
    lifecycle_generation: u64,
    cancel: Arc<AtomicBool>,
}

#[derive(Clone)]
struct ExternalScanAdmission {
    source_id: String,
    lifecycle_generation: u64,
}

struct ExternalScanRegistration {
    source_id: String,
    lifecycle_generation: u64,
    cancel: Arc<AtomicBool>,
}

#[derive(Clone)]
struct PendingSourceRetirement {
    source: SampleSource,
    lifecycle_generation: u64,
    retry_at: i64,
}

#[derive(Default)]
struct ExternalScanState {
    admissions: BTreeMap<u64, ExternalScanAdmission>,
    registrations: BTreeMap<u64, ExternalScanRegistration>,
}

struct InFlightWorkGuard<'a> {
    shared: &'a Shared,
    source_id: String,
    lifecycle_generation: u64,
}

impl Drop for InFlightWorkGuard<'_> {
    fn drop(&mut self) {
        let mut in_flight = self
            .shared
            .in_flight_work
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let key = (self.source_id.clone(), self.lifecycle_generation);
        if let Some(count) = in_flight.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                in_flight.remove(&key);
            }
        }
        drop(in_flight);
        let mut control = self.shared.control();
        if control.pending_retirements.values().any(|retirement| {
            retirement.source.id.as_str() == self.source_id
                && retirement.lifecycle_generation == self.lifecycle_generation
        }) {
            control.notify("retired_source_work_released");
            drop(control);
            self.shared.wake.notify_one();
        }
    }
}

impl SourceProcessingBudgetHandle {
    pub(in crate::native_app) fn acquire_scan(
        &self,
        source_id: &str,
    ) -> Option<SourceProcessingBudgetPermit> {
        {
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
        }
        // Hold the budget lock through admission publication. This closes the race where the
        // coordinator could acquire the scan lane after we inspected it but before it observed the
        // foreground reservation.
        let budgets = self.shared.budgets();
        let active_sources = budgets.active_sources();
        let (admission_id, admission_cancel, lifecycle_generation) = {
            let mut control = self.shared.control();
            if control.shutdown
                || self.shared.cancel.load(Ordering::Acquire)
                || !control.source_is_active(source_id)
                || control.processing_paused()
            {
                return None;
            }
            for active_source_id in active_sources {
                tracing::info!(
                    target: "wavecrate::source_processing",
                    source_id,
                    preempted_source_id = active_source_id.as_str(),
                    "Foreground source scan admission is reserving occupied processing capacity"
                );
                control.cancel_source_work(&active_source_id);
                control.mark_source_dirty(
                    &active_source_id,
                    "foreground_scan_preempted_background_work",
                );
            }
            let admission_cancel = Arc::clone(&control.source_work_cancels[source_id]);
            let lifecycle_generation = control.source_lifecycle_generations[source_id];
            if admission_cancel.load(Ordering::Acquire) {
                return None;
            }
            let admission_id = self
                .shared
                .next_external_scan_id
                .fetch_add(1, Ordering::Relaxed);
            let mut external_scans = self.shared.external_scans();
            external_scans.admissions.insert(
                admission_id,
                ExternalScanAdmission {
                    source_id: source_id.to_string(),
                    lifecycle_generation,
                },
            );
            (admission_id, admission_cancel, lifecycle_generation)
        };
        drop(budgets);
        self.shared.wake.notify_one();
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
                    || control.source_lifecycle_generations.get(source_id)
                        != Some(&lifecycle_generation)
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
                        lifecycle_generation,
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
                    lifecycle_generation,
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
                || control.source_lifecycle_generations.get(source_id)
                    != Some(&lifecycle_generation)
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
            || self.permit.as_ref().is_some_and(|permit| {
                control.source_lifecycle_generations.get(permit.source_id())
                    != Some(&self.lifecycle_generation)
            })
    }
}

impl Drop for SourceProcessingBudgetPermit {
    fn drop(&mut self) {
        let registration = self
            .shared
            .external_scans()
            .registrations
            .remove(&self.registration_id);
        self.shared.external_scan_wake.notify_all();
        if registration.as_ref().is_some_and(|registration| {
            self.shared
                .control()
                .pending_retirements
                .values()
                .any(|retirement| {
                    retirement.source.id.as_str() == registration.source_id
                        && retirement.lifecycle_generation == registration.lifecycle_generation
                })
        }) {
            let mut control = self.shared.control();
            control.notify("retired_external_scan_released");
            drop(control);
            self.shared.wake.notify_one();
        }
        if let Some(permit) = self.permit.take() {
            let source_id = permit.source_id().to_string();
            self.shared.budgets().release(permit);
            self.shared.budget_wake.notify_all();
            let mut control = self.shared.control();
            if control.source_is_active(&source_id)
                && control.source_lifecycle_generations.get(&source_id)
                    == Some(&self.lifecycle_generation)
            {
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
    #[cfg(test)]
    pub(in crate::native_app) fn start(sources: Vec<SampleSource>) -> Self {
        Self::start_with_playback_state(sources, false)
    }

    pub(in crate::native_app) fn start_with_worker_sender(
        sources: Vec<SampleSource>,
        worker_sender: Sender<GuiMessage>,
    ) -> Self {
        Self::start_with_playback_state_and_sender(sources, false, Some(worker_sender))
    }

    #[cfg(test)]
    fn start_with_playback_state(sources: Vec<SampleSource>, playback_active: bool) -> Self {
        Self::start_with_playback_state_and_sender(sources, playback_active, None)
    }

    fn start_with_playback_state_and_sender(
        sources: Vec<SampleSource>,
        playback_active: bool,
        worker_sender: Option<Sender<GuiMessage>>,
    ) -> Self {
        let shared = Arc::new(Shared::new(sources, worker_sender));
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
            shared: Arc::new(Shared::new(Vec::new(), None)),
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
            .filter_map(|source_id| {
                Some((
                    control.sources.get(source_id)?.clone(),
                    *control.source_lifecycle_generations.get(source_id)?,
                ))
            })
            .collect::<Vec<_>>();
        for source_id in &changed_source_ids {
            if let Some(cancel) = control.source_work_cancels.get(source_id) {
                cancel.store(true, Ordering::Release);
            }
        }
        for (source, lifecycle_generation) in retired_sources {
            let retirement_id = control.next_retirement_id;
            control.next_retirement_id = control.next_retirement_id.wrapping_add(1).max(1);
            control.pending_retirements.insert(
                retirement_id,
                PendingSourceRetirement {
                    source,
                    lifecycle_generation,
                    retry_at: 0,
                },
            );
        }
        let mut source_work_cancels = BTreeMap::new();
        let mut source_lifecycle_generations = BTreeMap::new();
        for (source_id, source) in &sources {
            let unchanged = control
                .sources
                .get(source_id)
                .is_some_and(|current| source_descriptors_match(current, source));
            let cancel = if unchanged {
                control.source_work_cancels.get(source_id).cloned()
            } else {
                None
            }
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
            let lifecycle_generation = if unchanged {
                control
                    .source_lifecycle_generations
                    .get(source_id)
                    .copied()
                    .unwrap_or_else(|| control.allocate_lifecycle_generation())
            } else {
                control.allocate_lifecycle_generation()
            };
            source_work_cancels.insert(source_id.clone(), cancel);
            source_lifecycle_generations.insert(source_id.clone(), lifecycle_generation);
        }
        control.sources = sources;
        control.source_work_cancels = source_work_cancels;
        control.source_lifecycle_generations = source_lifecycle_generations;
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
        self.shared.cancel_external_scans(|registration| {
            changed_source_ids.contains(&registration.source_id)
        });
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_all();
        Ok(())
    }

    /// Admit a newly configured source before its first external scan starts.
    ///
    /// This deliberately only grows the configured set. Full replacement also
    /// retires removed lifecycle epochs and is owned by the configuration path.
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
        let lifecycle_generation = control.allocate_lifecycle_generation();
        control
            .source_lifecycle_generations
            .insert(source_id.clone(), lifecycle_generation);
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

    pub(in crate::native_app) fn lifecycle_generations(&self) -> BTreeMap<String, u64> {
        self.shared.control().source_lifecycle_generations.clone()
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
                control.pause_feedback_pending = true;
            } else if !control.foreground_active {
                control.reset_source_work_tokens();
            }
            if active {
                control.notify("playback_pause");
            } else {
                control.notify("playback_resume");
            }
            if active {
                publish_source_processing_pausing(&self.shared, "Waiting for playback");
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
            control.pause_feedback_pending = true;
        } else if !control.playback_active {
            control.reset_source_work_tokens();
        }
        if active {
            control.notify("foreground_activity_pause");
        } else {
            control.notify("foreground_activity_resume");
        }
        if active {
            publish_source_processing_pausing(&self.shared, "Waiting for source loading");
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
    in_flight_work: Mutex<BTreeMap<(String, u64), usize>>,
    worker_sender: Option<Sender<GuiMessage>>,
}

impl Shared {
    fn new(sources: Vec<SampleSource>, worker_sender: Option<Sender<GuiMessage>>) -> Self {
        let sources = sources_by_id(sources);
        let source_work_cancels = sources
            .keys()
            .map(|source_id| (source_id.clone(), Arc::new(AtomicBool::new(false))))
            .collect();
        let mut next_lifecycle_generation = 1_u64;
        let source_lifecycle_generations = sources
            .keys()
            .map(|source_id| {
                let generation = next_lifecycle_generation;
                next_lifecycle_generation = next_lifecycle_generation.wrapping_add(1).max(1);
                (source_id.clone(), generation)
            })
            .collect();
        #[cfg(not(test))]
        let (pending_retirements, next_retirement_id) =
            match recovered_source_retirements(&sources, &mut next_lifecycle_generation) {
                Ok(recovered) => recovered,
                Err(error) => {
                    tracing::warn!(
                        target: "wavecrate::source_processing",
                        error,
                        "Retained source retirement recovery was deferred"
                    );
                    (BTreeMap::new(), 1)
                }
            };
        #[cfg(test)]
        let pending_retirements = BTreeMap::new();
        #[cfg(test)]
        let next_retirement_id = 1_u64;
        let dirty_sources = sources.keys().cloned().collect();
        Self {
            source_replacement: Mutex::new(()),
            state: Mutex::new(ControlState {
                sources,
                source_work_cancels,
                source_lifecycle_generations,
                next_lifecycle_generation,
                dirty_sources,
                quarantined_sources: BTreeSet::new(),
                pending_retirements,
                next_retirement_id,
                wake_generation: 1,
                wake_reason: "startup",
                playback_active: false,
                foreground_active: false,
                pause_feedback_pending: false,
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
            worker_sender,
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

    fn has_external_scan_admission(&self) -> bool {
        !self.external_scans().admissions.is_empty()
    }

    fn has_external_scan_activity(&self) -> bool {
        let scans = self.external_scans();
        !scans.admissions.is_empty() || !scans.registrations.is_empty()
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

    fn source_has_external_activity(&self, source_id: &str, lifecycle_generation: u64) -> bool {
        let scans = self.external_scans();
        scans.admissions.values().any(|admitted| {
            admitted.source_id == source_id && admitted.lifecycle_generation == lifecycle_generation
        }) || scans.registrations.values().any(|registration| {
            registration.source_id == source_id
                && registration.lifecycle_generation == lifecycle_generation
        })
    }

    fn finish_external_scan_admission(&self, admission_id: u64) {
        let mut external_scans = self.external_scans();
        let admission = external_scans.admissions.remove(&admission_id);
        drop(external_scans);
        self.external_scan_wake.notify_all();
        if admission.as_ref().is_some_and(|admission| {
            self.control()
                .pending_retirements
                .values()
                .any(|retirement| {
                    retirement.source.id.as_str() == admission.source_id
                        && retirement.lifecycle_generation == admission.lifecycle_generation
                })
        }) {
            let mut control = self.control();
            control.notify("retired_external_admission_released");
            drop(control);
            self.wake.notify_one();
        }
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
        let lifecycle_generation = control
            .source_lifecycle_generations
            .get(source_id)
            .copied()?;
        *in_flight
            .entry((source_id.to_string(), lifecycle_generation))
            .or_default() += 1;
        drop(in_flight);
        drop(control);
        Some(InFlightWorkGuard {
            shared: self,
            source_id: source_id.to_string(),
            lifecycle_generation,
        })
    }

    fn source_has_in_flight_work(&self, source_id: &str, lifecycle_generation: u64) -> bool {
        self.in_flight_work
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(&(source_id.to_string(), lifecycle_generation))
            .is_some_and(|count| *count > 0)
    }
}

struct ControlState {
    sources: BTreeMap<String, SampleSource>,
    source_work_cancels: BTreeMap<String, Arc<AtomicBool>>,
    source_lifecycle_generations: BTreeMap<String, u64>,
    next_lifecycle_generation: u64,
    dirty_sources: BTreeSet<String>,
    quarantined_sources: BTreeSet<String>,
    pending_retirements: BTreeMap<u64, PendingSourceRetirement>,
    next_retirement_id: u64,
    wake_generation: u64,
    wake_reason: &'static str,
    playback_active: bool,
    foreground_active: bool,
    pause_feedback_pending: bool,
    shutdown: bool,
    priority: PriorityContext,
}

impl ControlState {
    fn source_is_active(&self, source_id: &str) -> bool {
        let Some(source) = self.sources.get(source_id) else {
            return false;
        };
        !self.quarantined_sources.contains(source_id)
            && !self
                .pending_retirements
                .values()
                .any(|retirement| source_storage_identity_matches(source, &retirement.source))
    }

    fn notify(&mut self, reason: &'static str) {
        self.wake_generation = self.wake_generation.wrapping_add(1);
        self.wake_reason = reason;
    }

    fn allocate_lifecycle_generation(&mut self) -> u64 {
        let generation = self.next_lifecycle_generation;
        self.next_lifecycle_generation = self.next_lifecycle_generation.wrapping_add(1).max(1);
        generation
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
    ManifestAudit,
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
    progress_completed: usize,
    progress_total: usize,
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
    PrerequisiteInvalidated,
    Stale,
    Cancelled,
    Parked,
    NotClaimed,
}

impl ExecutionOutcome {
    fn was_claimed(self) -> bool {
        !matches!(self, Self::Parked | Self::NotClaimed)
    }
}

fn should_requeue_cancelled(
    outcome: Option<ExecutionOutcome>,
    source_active: bool,
    source_dirty: bool,
) -> bool {
    matches!(outcome, Some(ExecutionOutcome::Cancelled)) && source_active && !source_dirty
}

fn run_coordinator(shared: Arc<Shared>) {
    let mut observed_generation = 0;
    let mut next_retry_at = None;
    let mut next_safety_sweep_at = Instant::now() + SAFETY_SWEEP_INTERVAL;
    let mut scheduler = FairScheduler::default();
    let mut reset_sources = BTreeMap::<String, bool>::new();
    let mut candidates = Vec::<RuntimeCandidate>::new();
    let mut source_stats = BTreeMap::<String, SourceDiscoveryStats>::new();
    let mut displayed_source_stats = BTreeMap::<String, SourceDiscoveryStats>::new();
    let mut active_progress_source = None::<String>;
    let mut last_progress_publish_at = None::<Instant>;
    let mut pending_similarity_refresh_sources = BTreeSet::<String>::new();
    let mut last_similarity_refresh_publish_at = None::<Instant>;
    let mut progress_visible = false;
    loop {
        process_ready_source_retirements(&shared);
        let (
            sources,
            dirty_sources,
            source_work_cancels,
            processing_paused,
            pause_feedback_pending,
            generation,
            reason,
        ) = {
            let mut control = shared.control();
            while !control.shutdown && control.wake_generation == observed_generation {
                let pending_retirement_retry_at = control
                    .pending_retirements
                    .values()
                    .filter_map(|retirement| {
                        (retirement.retry_at > 0).then_some(retirement.retry_at)
                    })
                    .min();
                let wait_duration = coordinator_wait_duration(
                    earliest_deadline(next_retry_at, pending_retirement_retry_at),
                    now_epoch_seconds(),
                    next_safety_sweep_at.saturating_duration_since(Instant::now()),
                    control.processing_paused(),
                );
                if progress_visible && !wait_duration.is_zero() {
                    // Keep feedback stable across immediate coordinator handoffs. Only clear it
                    // when the coordinator is genuinely about to sleep with no newly published
                    // work waiting to be handled.
                    publish_source_processing_finished(&shared);
                    progress_visible = false;
                    active_progress_source = None;
                    last_progress_publish_at = None;
                    displayed_source_stats.clear();
                }
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
            let pause_feedback_pending = std::mem::take(&mut control.pause_feedback_pending);
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
                pause_feedback_pending,
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
        displayed_source_stats.retain(|source_id, _| configured_source_ids.contains(source_id));
        if pause_feedback_pending {
            // The pause may already have resumed by the time a long-running database
            // checkpoint returns. Acknowledge the latched transition here so rapid foreground
            // activity cannot leave stale pausing feedback behind.
            publish_source_processing_finished(&shared);
            progress_visible = false;
            active_progress_source = None;
            last_progress_publish_at = None;
            displayed_source_stats.clear();
        }
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
                match reset_interrupted_readiness_jobs(source) {
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
        if publish_source_processing_discovery_if_needed(
            &shared,
            &sources_to_discover,
            progress_visible,
        ) {
            progress_visible = true;
        }
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
            let Some(schedule_index) = scheduler.choose(&schedules, &priority, &shared.budgets())
            else {
                let mut telemetry = shared.telemetry();
                telemetry.contention = telemetry.contention.saturating_add(1);
                break;
            };
            let index = eligible_indices[schedule_index];
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
            let progress_publish_due = last_progress_publish_at
                .is_none_or(|published_at| published_at.elapsed() >= PROGRESS_REFRESH_INTERVAL);
            if (active_progress_source.as_deref() != Some(candidate.source.id.as_str())
                || !matches!(&candidate.task, RuntimeTask::Readiness(_))
                || progress_publish_due)
                && let Some(progress) = source_stats.get(candidate.source.id.as_str()).copied()
            {
                let progress = stable_source_progress(
                    &mut displayed_source_stats,
                    candidate.source.id.as_str(),
                    progress,
                );
                publish_source_processing_progress(&shared, &candidate, progress);
                active_progress_source = Some(candidate.source.id.as_str().to_string());
                last_progress_publish_at = Some(Instant::now());
                progress_visible = true;
            }
            let result = execute_candidate(
                &candidate,
                in_flight_work.lifecycle_generation,
                candidate_cancel.as_ref(),
                shared.worker_sender.as_ref(),
            );
            drop(in_flight_work);
            shared.budgets().release(permit);
            shared.budget_wake.notify_all();
            let mut telemetry = shared.telemetry();
            let mut execution_outcome = None;
            match result {
                Ok(outcome) => {
                    execution_outcome = Some(outcome);
                    if outcome == ExecutionOutcome::Completed
                        && let RuntimeTask::Readiness(target) = &candidate.task
                        && target.stage == ReadinessStage::EmbeddingAspects
                    {
                        pending_similarity_refresh_sources.insert(target.source_id.clone());
                    }
                    if outcome.was_claimed() {
                        telemetry.claimed = telemetry.claimed.saturating_add(1);
                    }
                    match outcome {
                        ExecutionOutcome::Completed => {
                            telemetry.completed = telemetry.completed.saturating_add(1);
                            if matches!(&candidate.task, RuntimeTask::Readiness(_))
                                && let Some(progress) = advance_source_progress(
                                    &mut source_stats,
                                    candidate.source.id.as_str(),
                                )
                                && progress_refresh_due(last_progress_publish_at)
                            {
                                let progress = stable_source_progress(
                                    &mut displayed_source_stats,
                                    candidate.source.id.as_str(),
                                    progress,
                                );
                                publish_source_processing_progress(&shared, &candidate, progress);
                                last_progress_publish_at = Some(Instant::now());
                                progress_visible = true;
                            }
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
                            telemetry.failed = telemetry.failed.saturating_add(1);
                            if matches!(&candidate.task, RuntimeTask::Readiness(_))
                                && let Some(progress) = advance_source_progress(
                                    &mut source_stats,
                                    candidate.source.id.as_str(),
                                )
                                && progress_refresh_due(last_progress_publish_at)
                            {
                                let progress = stable_source_progress(
                                    &mut displayed_source_stats,
                                    candidate.source.id.as_str(),
                                    progress,
                                );
                                publish_source_processing_progress(&shared, &candidate, progress);
                                last_progress_publish_at = Some(Instant::now());
                                progress_visible = true;
                            }
                        }
                        ExecutionOutcome::PrerequisiteInvalidated => {
                            telemetry.stale = telemetry.stale.saturating_add(1);
                            shared.control().mark_source_dirty(
                                candidate.source.id.as_str(),
                                "readiness_prerequisite_invalidated",
                            );
                        }
                        ExecutionOutcome::Stale => {
                            telemetry.stale = telemetry.stale.saturating_add(1)
                        }
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
            drop(telemetry);
            if last_similarity_refresh_publish_at.is_none_or(|published_at| {
                published_at.elapsed() >= SIMILARITY_SCORE_REFRESH_INTERVAL
            }) && publish_similarity_readiness_refreshes(
                &shared,
                &mut pending_similarity_refresh_sources,
            ) {
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
            let source_id = candidate.source.id.as_str();
            if candidates
                .iter()
                .any(|queued| queued.source.id.as_str() == source_id)
            {
                continue;
            }
            let should_refresh = match (&candidate.task, execution_outcome) {
                (RuntimeTask::ManifestAudit, Some(ExecutionOutcome::Completed))
                | (RuntimeTask::ManifestAudit, Some(ExecutionOutcome::Failed))
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
        if publish_similarity_readiness_refreshes(&shared, &mut pending_similarity_refresh_sources)
        {
            last_similarity_refresh_publish_at = Some(Instant::now());
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

fn process_ready_source_retirements(shared: &Shared) {
    let now = now_epoch_seconds();
    let candidates = {
        let control = shared.control();
        control
            .pending_retirements
            .iter()
            .filter(|(_, retirement)| retirement.retry_at <= now)
            .map(|(retirement_id, retirement)| (*retirement_id, retirement.clone()))
            .collect::<Vec<_>>()
    };

    for (retirement_id, retirement) in candidates {
        // Serialize the final admission check and the short retirement transaction with source
        // replacement. Removal itself only enqueues this work; a fast re-add can therefore
        // supersede it before any durable state is changed.
        let replacement = shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let control = shared.control();
        let Some(current) = control.pending_retirements.get(&retirement_id) else {
            continue;
        };
        if current.lifecycle_generation != retirement.lifecycle_generation {
            continue;
        }
        drop(control);
        if shared.source_has_external_activity(
            retirement.source.id.as_str(),
            retirement.lifecycle_generation,
        ) || shared.source_has_in_flight_work(
            retirement.source.id.as_str(),
            retirement.lifecycle_generation,
        ) {
            continue;
        }
        let mut control = shared.control();
        let Some(current) = control.pending_retirements.get(&retirement_id) else {
            continue;
        };
        if current.lifecycle_generation != retirement.lifecycle_generation {
            continue;
        }
        let reactivated_source_id = control.sources.values().find_map(|active| {
            source_storage_identity_matches(active, &retirement.source)
                .then(|| active.id.as_str().to_string())
        });
        if let Some(source_id) = reactivated_source_id {
            control.pending_retirements.remove(&retirement_id);
            control.dirty_sources.insert(source_id);
            control.notify("source_storage_handoff_completed");
            continue;
        }
        drop(control);
        let result = retire_source_derived_state(&retirement.source);
        let mut control = shared.control();
        match result {
            Ok(retired_cache_refs) => {
                control.pending_retirements.remove(&retirement_id);
                tracing::info!(
                    target: "wavecrate::source_processing",
                    source_id = retirement.source.id.as_str(),
                    retired_cache_refs,
                    "Retired removed source runtime and path-derived cache ownership"
                );
                drop(control);
                drop(replacement);
                if let Err(error) = prune_unreferenced_waveform_cache() {
                    tracing::warn!(
                        target: "wavecrate::source_processing",
                        source_id = retirement.source.id.as_str(),
                        error,
                        "Bounded orphan cache collection was deferred"
                    );
                }
            }
            Err(error) => {
                if let Some(pending) = control.pending_retirements.get_mut(&retirement_id) {
                    pending.retry_at = now.saturating_add(SOURCE_RETIREMENT_RETRY_SECONDS);
                }
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    source_id = retirement.source.id.as_str(),
                    error,
                    "Removed source retirement will retry without reactivating the source"
                );
            }
        }
    }
}

fn progress_refresh_due(last_publish_at: Option<Instant>) -> bool {
    last_publish_at.is_none_or(|published_at| published_at.elapsed() >= PROGRESS_REFRESH_INTERVAL)
}

fn publish_similarity_readiness_refreshes(
    shared: &Shared,
    pending_source_ids: &mut BTreeSet<String>,
) -> bool {
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return false;
    };
    if pending_source_ids.is_empty() {
        return false;
    }
    for source_id in std::mem::take(pending_source_ids) {
        let _ = worker_sender.send(GuiMessage::SimilarityReadinessAdvanced { source_id });
    }
    true
}

fn publish_source_processing_progress(
    shared: &Shared,
    candidate: &RuntimeCandidate,
    stats: SourceDiscoveryStats,
) {
    let control = shared.control();
    if control.processing_paused() {
        return;
    }
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return;
    };
    let (stage, detail) = runtime_task_progress_detail(&candidate.task);
    let (completed, total) = match &candidate.task {
        RuntimeTask::Readiness(_) => (stats.progress_completed, stats.progress_total),
        RuntimeTask::ManifestAudit => (0, 0),
    };
    let (completed, total) = if total > 0 && completed < total {
        (completed, total)
    } else {
        // A claimed candidate is active even when discovery counters have reached their current
        // boundary. Keep showing activity until the coordinator actually becomes idle instead of
        // publishing a false completion while the candidate is still executing.
        (0, 0)
    };
    let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
        SourceProcessingProgress {
            source_id: candidate.source.id.as_str().to_string(),
            lifecycle_generation: control
                .source_lifecycle_generations
                .get(candidate.source.id.as_str())
                .copied()
                .unwrap_or(0),
            active: true,
            completed,
            total,
            stage: stage.to_string(),
            detail,
        },
    ));
}

fn publish_source_processing_discovery(shared: &Shared, source: &SampleSource) {
    let control = shared.control();
    if control.processing_paused() {
        return;
    }
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return;
    };
    let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
        SourceProcessingProgress {
            source_id: source.id.as_str().to_string(),
            lifecycle_generation: control
                .source_lifecycle_generations
                .get(source.id.as_str())
                .copied()
                .unwrap_or(0),
            active: true,
            completed: 0,
            total: 0,
            stage: String::from("Checking pending work"),
            detail: String::from("Counting unfinished analysis, similarity, and indexing jobs"),
        },
    ));
}

fn publish_source_processing_discovery_if_needed(
    shared: &Shared,
    sources: &[SampleSource],
    progress_visible: bool,
) -> bool {
    if progress_visible || sources.is_empty() || shared.has_external_scan_activity() {
        return false;
    }
    if sources.len() == 1 {
        publish_source_processing_discovery(shared, &sources[0]);
    } else {
        publish_multi_source_processing_activity(shared, sources.len());
    }
    true
}

fn publish_multi_source_processing_activity(shared: &Shared, source_count: usize) {
    let control = shared.control();
    if control.processing_paused() {
        return;
    }
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return;
    };
    let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
        SourceProcessingProgress {
            source_id: String::new(),
            lifecycle_generation: 0,
            active: true,
            completed: 0,
            total: 0,
            stage: String::from("Processing source libraries"),
            detail: format!(
                "Advancing {source_count} source{}",
                if source_count == 1 { "" } else { "s" }
            ),
        },
    ));
}

fn publish_source_processing_finished(shared: &Shared) {
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return;
    };
    let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
        SourceProcessingProgress {
            source_id: String::new(),
            lifecycle_generation: 0,
            active: false,
            completed: 0,
            total: 0,
            stage: String::new(),
            detail: String::new(),
        },
    ));
}

fn publish_source_processing_pausing(shared: &Shared, detail: &str) {
    let Some(worker_sender) = shared.worker_sender.as_ref() else {
        return;
    };
    let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
        SourceProcessingProgress {
            source_id: String::new(),
            lifecycle_generation: 0,
            active: true,
            completed: 0,
            total: 0,
            stage: String::from("Pausing source processing"),
            detail: detail.to_string(),
        },
    ));
}

fn advance_source_progress(
    source_stats: &mut BTreeMap<String, SourceDiscoveryStats>,
    source_id: &str,
) -> Option<SourceDiscoveryStats> {
    let stats = source_stats.get_mut(source_id)?;
    stats.progress_completed = stats
        .progress_completed
        .saturating_add(1)
        .min(stats.progress_total);
    Some(*stats)
}

fn stable_source_progress(
    displayed: &mut BTreeMap<String, SourceDiscoveryStats>,
    source_id: &str,
    observed: SourceDiscoveryStats,
) -> SourceDiscoveryStats {
    let stable = displayed.entry(source_id.to_string()).or_insert(observed);
    if stable.progress_total != observed.progress_total {
        *stable = observed;
        return *stable;
    }
    stable.progress_completed = stable
        .progress_completed
        .max(observed.progress_completed)
        .min(stable.progress_total);
    *stable
}

fn runtime_task_progress_detail(task: &RuntimeTask) -> (&'static str, String) {
    match task {
        RuntimeTask::Readiness(target) => readiness_progress_detail(target),
        RuntimeTask::ManifestAudit => (
            "Scanning source changes",
            String::from("Checking the source manifest"),
        ),
    }
}

fn readiness_progress_detail(target: &ReadinessTarget) -> (&'static str, String) {
    let stage = match target.stage {
        ReadinessStage::IndexedIdentity => "Indexing files",
        ReadinessStage::PlaybackSummary => "Preparing playback",
        ReadinessStage::AnalysisFeatures => "Analyzing audio",
        ReadinessStage::EmbeddingAspects => "Preparing similarity",
        ReadinessStage::SimilarityLayout => "Building similarity layout",
    };
    let detail = target
        .relative_path
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| String::from("Finalizing source"));
    (stage, detail)
}

fn scheduler_candidate_indices(
    candidates: &[RuntimeCandidate],
    external_scan_admitted: bool,
) -> Vec<usize> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, _candidate)| (!external_scan_admitted).then_some(index))
        .collect()
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
    if !source.root.is_dir() {
        if database_root != source.root && database_root.is_dir() {
            let connection = SourceDatabase::open_unavailable_source_metadata_connection(
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            )
            .map_err(|error| error.to_string())?;
            if source_processing_schema_available(&connection)? {
                mark_readiness_temporarily_unavailable(&connection, source.id.as_str(), now)?;
            }
        }
        return Ok(Cancellable::Completed((
            Vec::new(),
            SourceDiscoveryStats::default(),
        )));
    }
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
    let pruned_legacy_jobs = prune_legacy_similarity_jobs(connection)?;
    if pruned_legacy_jobs > 0 {
        tracing::info!(
            target: "wavecrate::source_processing",
            source_id,
            pruned_legacy_jobs,
            "Removed jobs owned by the retired similarity pipeline"
        );
    }
    let last_manifest_audit_at = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            [META_LAST_MANIFEST_AUDIT_AT],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_default();
    let manifest_identity_repair_due: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM wav_files
                WHERE missing = 0
                  AND (file_identity IS NULL OR TRIM(file_identity) = '')
                  AND path NOT GLOB '._*'
                  AND path NOT GLOB '*/._*'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if manifest_identity_repair_due
        || now.saturating_sub(last_manifest_audit_at) >= MANIFEST_AUDIT_INTERVAL_SECONDS
    {
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(source_id, ProcessingLane::Scan, 0, now),
            source: source.clone(),
            task: RuntimeTask::ManifestAudit,
        });
    }
    if matches!(
        publish_current_readiness_targets_with_cancel(connection, source_id, now, cancel)?,
        Cancellable::Cancelled
    ) {
        return Ok(Cancellable::Cancelled);
    }
    if matches!(
        reconcile_playback_cache_ownership(source, connection, cancel)?,
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
        let reclassified = reclassify_known_unsupported_audio_failures(connection)?;
        if reclassified > 0 {
            tracing::info!(
                target: "wavecrate::source_processing",
                source_id,
                reclassified,
                "Reclassified deterministic audio decode failures as unsupported"
            );
        }
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
        stats.progress_total = work_stats.total;
        stats.progress_completed = work_stats
            .completed
            .saturating_add(work_stats.permanent_failures)
            .saturating_add(work_stats.unsupported)
            .min(stats.progress_total);
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

    if cancelled(cancel) {
        Ok(Cancellable::Cancelled)
    } else {
        Ok(Cancellable::Completed((candidates, stats)))
    }
}

fn prune_legacy_similarity_jobs(connection: &rusqlite::Connection) -> Result<usize, String> {
    let removed = connection
        .execute(
            "DELETE FROM analysis_jobs
             WHERE readiness_managed = 0
               AND job_type IN ('wav_metadata_v1', 'embedding_backfill_v1', 'rebuild_index_v1')",
            [],
        )
        .map_err(|error| format!("Prune retired similarity jobs failed: {error}"))?;
    connection
        .execute(
            "DELETE FROM analysis_job_progress_snapshots
             WHERE job_type IN ('wav_metadata_v1', 'embedding_backfill_v1', 'rebuild_index_v1')",
            [],
        )
        .map_err(|error| format!("Prune retired similarity progress failed: {error}"))?;
    Ok(removed)
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
        (
            "source_readiness_artifacts",
            &[
                "source_id",
                "scope_kind",
                "scope_id",
                "relative_path",
                "stage",
                "artifact_version",
                "content_generation",
                "artifact_ref",
            ][..],
        ),
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

fn retire_source_derived_state(source: &SampleSource) -> Result<usize, String> {
    let database_path = source.db_path().map_err(|error| error.to_string())?;
    if !database_path.exists() {
        if source.metadata_storage == SourceMetadataStorage::SourceFolder && !source.root.is_dir() {
            return Err(format!(
                "source storage is unavailable: {}",
                source.root.display()
            ));
        }
        return Ok(0);
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
        return Err(format!(
            "source database is read-only: {}",
            database_path.display()
        ));
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
        return Ok(0);
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let cache_refs = {
        let mut statement = transaction
            .prepare(
                "SELECT artifact_ref
                 FROM source_readiness_artifacts
                 WHERE source_id = ?1
                   AND stage = 'playback_summary'
                   AND artifact_ref IS NOT NULL
                   AND length(trim(artifact_ref)) > 0",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([source.id.as_str()], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
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
    transaction
        .execute(
            "DELETE FROM analysis_jobs
             WHERE source_id = ?1 AND readiness_managed = 1",
            [source.id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM source_readiness_artifacts
             WHERE source_id = ?1 AND stage = 'playback_summary'",
            [source.id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM source_readiness_targets
             WHERE source_id = ?1 AND stage = 'playback_summary'",
            [source.id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    let mut invalidated = 0;
    for cache_ref in &cache_refs {
        match retained_waveform_cache_ref_is_owned(cache_ref) {
            Ok(false) => {
                invalidate_persisted_waveform_cache_ref(std::path::Path::new(cache_ref));
                invalidated += 1;
            }
            Ok(true) => {}
            Err(error) => tracing::warn!(
                target: "wavecrate::source_processing",
                cache_ref,
                error,
                "Retained cache ownership could not be proven; payload was preserved"
            ),
        }
    }
    Ok(invalidated)
}

fn retained_waveform_cache_ref_is_owned(cache_ref: &str) -> Result<bool, String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    let mut visited = BTreeSet::new();
    for retained in retained_sources {
        let database_path = retained.db_path().map_err(|error| error.to_string())?;
        if !visited.insert(database_path.clone()) {
            continue;
        }
        if !database_path.is_file() {
            return Err(format!(
                "retained source database is unavailable: {}",
                database_path.display()
            ));
        }
        let connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| error.to_string())?;
        let owned = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1
                    FROM source_readiness_artifacts
                    WHERE stage = 'playback_summary' AND artifact_ref = ?1
                 )",
                [cache_ref],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or(false);
        if owned {
            return Ok(true);
        }
    }
    Ok(false)
}

fn prune_unreferenced_waveform_cache() -> Result<usize, String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    if retained_sources.len() > RETAINED_SOURCE_MAX_SCANNED {
        return Err(format!(
            "retained source count {} exceeds bounded GC scan limit {RETAINED_SOURCE_MAX_SCANNED}",
            retained_sources.len()
        ));
    }
    let mut referenced = BTreeSet::<PathBuf>::new();
    for source in retained_sources {
        let database_path = source.db_path().map_err(|error| error.to_string())?;
        if !database_path.is_file() {
            return Err(format!(
                "retained source database is unavailable: {}",
                database_path.display()
            ));
        }
        let connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| format!("open retained cache manifest: {error}"))?;
        let manifest_exists = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = 'source_readiness_artifacts'
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| error.to_string())?;
        if !manifest_exists {
            continue;
        }
        let mut statement = connection
            .prepare(
                "SELECT artifact_ref
                 FROM source_readiness_artifacts
                 WHERE stage = 'playback_summary'
                   AND artifact_ref IS NOT NULL
                   AND length(trim(artifact_ref)) > 0",
            )
            .map_err(|error| error.to_string())?;
        let refs = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        referenced.extend(refs.into_iter().map(PathBuf::from));
    }

    let cache_dir = wavecrate::app_dirs::waveform_cache_dir().map_err(|error| error.to_string())?;
    let entries = match fs::read_dir(&cache_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.to_string()),
    };
    let cutoff = SystemTime::now()
        .checked_sub(ORPHAN_CACHE_MIN_AGE)
        .unwrap_or(UNIX_EPOCH);
    let mut removed = 0_usize;
    let cursor = ORPHAN_CACHE_SCAN_CURSOR.load(Ordering::Relaxed);
    let mut scanned = 0_usize;
    let mut delete_limit_reached = false;
    for entry in entries
        .flatten()
        .skip(cursor)
        .take(ORPHAN_CACHE_MAX_SCANNED)
    {
        if removed >= ORPHAN_CACHE_MAX_REMOVED {
            delete_limit_reached = true;
            break;
        }
        scanned = scanned.saturating_add(1);
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "wfc") || referenced.contains(&path)
        {
            continue;
        }
        let old_enough = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .is_some_and(|modified| modified <= cutoff);
        if !old_enough {
            continue;
        }
        invalidate_persisted_waveform_cache_ref(&path);
        if !path.exists() {
            removed = removed.saturating_add(1);
        }
    }
    ORPHAN_CACHE_SCAN_CURSOR.store(
        if scanned < ORPHAN_CACHE_MAX_SCANNED && !delete_limit_reached {
            0
        } else {
            cursor.saturating_add(scanned)
        },
        Ordering::Relaxed,
    );
    Ok(removed)
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
    let unsupported_generations = readiness_unsupported_content_generations(connection, source_id)?;
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
        let content_hash = content_hash.filter(|value| !value.trim().is_empty());
        let content_generation = content_hash
            .clone()
            .unwrap_or_else(|| format!("pending-{identity}-{file_size}-{modified_ns}"));
        manifest.push((path, identity, content_hash, content_generation, file_size));
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
    let similarity_artifact_version = native_similarity_artifact_version();
    let mut membership = blake3::Hasher::new();
    let mut targets = Vec::with_capacity(manifest.len().saturating_mul(4).saturating_add(1));
    for (path, identity, content_hash, content_generation, file_size) in &manifest {
        checkpoint();
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        let analyzable = *file_size > 0;
        let unsupported = content_hash.as_ref().is_some_and(|content_hash| {
            unsupported_generations.contains(&(identity.clone(), content_hash.clone()))
        });
        if analyzable && !unsupported {
            membership.update(identity.as_bytes());
            membership.update(&[0]);
            membership.update(content_generation.as_bytes());
            membership.update(&[0xff]);
        }
        targets.push(ReadinessTarget::file(
            source_id,
            identity,
            path,
            ReadinessStage::IndexedIdentity,
            READINESS_MANIFEST_VERSION,
            source_generation,
            content_generation,
        ));
        for (stage, version) in [
            (ReadinessStage::PlaybackSummary, READINESS_PLAYBACK_VERSION),
            (
                ReadinessStage::AnalysisFeatures,
                wavecrate_analysis::analysis_version(),
            ),
            (ReadinessStage::EmbeddingAspects, embedding_version.as_str()),
        ] {
            let mut target = ReadinessTarget::file(
                source_id,
                identity,
                path,
                stage,
                version,
                source_generation,
                content_generation,
            );
            if !analyzable || unsupported {
                target = target.with_eligibility(ReadinessEligibility::Unsupported);
            }
            targets.push(target);
        }
    }
    let membership_generation = membership.finalize().to_hex().to_string();
    targets.push(ReadinessTarget::source(
        source_id,
        ReadinessStage::SimilarityLayout,
        &similarity_artifact_version,
        source_generation,
        membership_generation.as_str(),
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
    let similarity_state = serde_json::json!({
        "state": "dirty",
        "source_generation": source_generation,
        "membership_generation": membership_generation,
        "artifact_version": similarity_artifact_version,
    })
    .to_string();
    wavecrate_analysis::ann_index::mark_artifacts_dirty(connection, &similarity_state)?;
    connection
        .execute(
            "INSERT INTO metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_READINESS_TARGET_FINGERPRINT, target_fingerprint],
        )
        .map_err(|error| error.to_string())?;
    Ok(Cancellable::Completed(true))
}

#[derive(Debug)]
struct PlaybackCacheOwnershipRow {
    scope_id: String,
    artifact_relative_path: Option<String>,
    artifact_version: String,
    artifact_content_generation: String,
    artifact_ref: Option<String>,
    target_relative_path: Option<String>,
    target_version: Option<String>,
    target_content_generation: Option<String>,
    target_eligibility: Option<String>,
}

/// Reconcile source-owned playback artifact rows with their exact app-global cache payloads.
///
/// The source database is the durable reverse index. This pass runs on startup and every committed
/// source wake, so changed/deleted identities can retire their old cache keys even after the old
/// filesystem metadata is gone. Path-only moves remap reusable payloads without decoding again.
fn reconcile_playback_cache_ownership(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    cancel: &AtomicBool,
) -> Result<Cancellable<usize>, String> {
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT artifact.scope_id,
                        artifact.relative_path,
                        artifact.artifact_version,
                        artifact.content_generation,
                        artifact.artifact_ref,
                        target.relative_path,
                        target.required_version,
                        target.content_generation,
                        target.eligibility
                 FROM source_readiness_artifacts AS artifact
                 LEFT JOIN source_readiness_targets AS target
                   ON target.source_id = artifact.source_id
                  AND target.scope_kind = artifact.scope_kind
                  AND target.scope_id = artifact.scope_id
                  AND target.stage = artifact.stage
                 WHERE artifact.source_id = ?1
                   AND artifact.scope_kind = 'file'
                   AND artifact.stage = 'playback_summary'
                 ORDER BY artifact.scope_id",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([source.id.as_str()], |row| {
                Ok(PlaybackCacheOwnershipRow {
                    scope_id: row.get(0)?,
                    artifact_relative_path: row.get(1)?,
                    artifact_version: row.get(2)?,
                    artifact_content_generation: row.get(3)?,
                    artifact_ref: row.get(4)?,
                    target_relative_path: row.get(5)?,
                    target_version: row.get(6)?,
                    target_content_generation: row.get(7)?,
                    target_eligibility: row.get(8)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    let mut changed = 0_usize;
    for row in rows {
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        let target_matches = row.target_version.as_deref() == Some(row.artifact_version.as_str())
            && row.target_content_generation.as_deref()
                == Some(row.artifact_content_generation.as_str())
            && row.target_eligibility.as_deref() == Some("eligible");
        let Some(cache_ref) = row.artifact_ref.as_deref().map(std::path::Path::new) else {
            changed = changed.saturating_add(delete_playback_cache_ownership_row(
                connection,
                source.id.as_str(),
                &row,
            )?);
            continue;
        };
        if !target_matches {
            invalidate_playback_cache_owner(source, &row);
            changed = changed.saturating_add(delete_playback_cache_ownership_row(
                connection,
                source.id.as_str(),
                &row,
            )?);
            continue;
        }
        let (Some(artifact_relative_path), Some(target_relative_path)) = (
            row.artifact_relative_path.as_deref(),
            row.target_relative_path.as_deref(),
        ) else {
            invalidate_playback_cache_owner(source, &row);
            changed = changed.saturating_add(delete_playback_cache_ownership_row(
                connection,
                source.id.as_str(),
                &row,
            )?);
            continue;
        };
        let target_path = source.root.join(target_relative_path);
        if artifact_relative_path == target_relative_path
            && persisted_waveform_cache_ref_is_current(&target_path, cache_ref)
        {
            continue;
        }
        if artifact_relative_path != target_relative_path {
            let artifact_path = source.root.join(artifact_relative_path);
            invalidate_persisted_waveform_cache_path(&artifact_path);
            if let Some(remapped_ref) = remap_persisted_waveform_cache_ref_after_move(
                cache_ref,
                &artifact_path,
                &target_path,
            ) {
                let updated = connection
                    .execute(
                        "UPDATE source_readiness_artifacts AS artifact
                         SET relative_path = ?1, artifact_ref = ?2
                         WHERE artifact.source_id = ?3
                           AND artifact.scope_kind = 'file'
                           AND artifact.scope_id = ?4
                           AND artifact.stage = 'playback_summary'
                           AND artifact.artifact_version = ?5
                           AND artifact.content_generation = ?6
                           AND artifact.artifact_ref IS ?7
                           AND EXISTS (
                               SELECT 1 FROM source_readiness_targets AS target
                               WHERE target.source_id = artifact.source_id
                                 AND target.scope_kind = artifact.scope_kind
                                 AND target.scope_id = artifact.scope_id
                                 AND target.stage = artifact.stage
                                 AND target.relative_path = ?1
                                 AND target.required_version = artifact.artifact_version
                                 AND target.content_generation = artifact.content_generation
                                 AND target.eligibility = 'eligible'
                           )",
                        params![
                            target_relative_path,
                            remapped_ref.to_string_lossy(),
                            source.id.as_str(),
                            row.scope_id,
                            row.artifact_version,
                            row.artifact_content_generation,
                            row.artifact_ref,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                if updated == 1 {
                    changed = changed.saturating_add(1);
                    continue;
                }
                invalidate_persisted_waveform_cache_ref(&remapped_ref);
            }
        } else {
            invalidate_playback_cache_owner(source, &row);
        }
        changed = changed.saturating_add(delete_playback_cache_ownership_row(
            connection,
            source.id.as_str(),
            &row,
        )?);
    }
    Ok(Cancellable::Completed(changed))
}

fn invalidate_playback_cache_owner(source: &SampleSource, row: &PlaybackCacheOwnershipRow) {
    if let Some(relative_path) = row.artifact_relative_path.as_deref() {
        invalidate_persisted_waveform_cache_path(&source.root.join(relative_path));
    }
    if let Some(cache_ref) = row.artifact_ref.as_deref() {
        invalidate_persisted_waveform_cache_ref(std::path::Path::new(cache_ref));
    }
}

fn delete_playback_cache_ownership_row(
    connection: &rusqlite::Connection,
    source_id: &str,
    row: &PlaybackCacheOwnershipRow,
) -> Result<usize, String> {
    connection
        .execute(
            "DELETE FROM source_readiness_artifacts
             WHERE source_id = ?1
               AND scope_kind = 'file'
               AND scope_id = ?2
               AND stage = 'playback_summary'
               AND artifact_version = ?3
               AND content_generation = ?4
               AND artifact_ref IS ?5",
            params![
                source_id,
                row.scope_id,
                row.artifact_version,
                row.artifact_content_generation,
                row.artifact_ref,
            ],
        )
        .map_err(|error| error.to_string())
}

fn readiness_unsupported_content_generations(
    connection: &rusqlite::Connection,
    source_id: &str,
) -> Result<BTreeSet<(String, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT readiness_scope_id, content_generation
             FROM analysis_jobs
             WHERE source_id = ?1
               AND readiness_managed = 1
               AND readiness_scope_kind = 'file'
               AND status = 'failed'
               AND failure_kind = 'unsupported'
               AND readiness_scope_id IS NOT NULL
               AND content_generation IS NOT NULL",
        )
        .map_err(|error| error.to_string())?;
    statement
        .query_map([source_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| error.to_string())?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|error| error.to_string())
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
        hash.update(&[0]);
        let eligibility: &[u8] = match target.eligibility {
            ReadinessEligibility::Eligible => b"eligible",
            ReadinessEligibility::Unsupported => b"unsupported",
            ReadinessEligibility::Deleted => b"deleted",
        };
        hash.update(eligibility);
        hash.update(&[0xff]);
    }
    Some(hash.finalize().to_hex().to_string())
}

fn execute_candidate(
    candidate: &RuntimeCandidate,
    lifecycle_generation: u64,
    cancel: &AtomicBool,
    worker_sender: Option<&Sender<GuiMessage>>,
) -> Result<ExecutionOutcome, String> {
    let result = match &candidate.task {
        RuntimeTask::ManifestAudit => {
            let database_root = candidate
                .source
                .database_root()
                .map_err(|error| error.to_string())?;
            if !candidate.source.root.is_dir() {
                tracing::info!(
                    target: "wavecrate::source_processing",
                    source_id = candidate.source.id.as_str(),
                    root = %candidate.source.root.display(),
                    "Skipping manifest audit because the source root became unavailable"
                );
                return Ok(ExecutionOutcome::Parked);
            }
            let database = SourceDatabase::open_for_background_job_with_database_root(
                &candidate.source.root,
                database_root,
            )
            .map_err(|error| error.to_string())?;
            let completed_at = now_epoch_seconds();
            let expected_files = database
                .list_manifest_entries()
                .map_err(|error| error.to_string())?
                .len();
            let source_id = candidate.source.id.as_str().to_string();
            let source_root = candidate.source.root.clone();
            let mut last_progress_publish_at = None::<Instant>;
            let mut publish_progress = |checked: usize, path: &std::path::Path| {
                let publish_due = last_progress_publish_at.is_none_or(|published_at| {
                    published_at.elapsed() >= Duration::from_millis(250)
                });
                if !publish_due {
                    return;
                }
                let Some(worker_sender) = worker_sender else {
                    return;
                };
                let relative = path.strip_prefix(&source_root).unwrap_or(path);
                let source_detail = if relative.as_os_str().is_empty() {
                    format!("Resumed after {checked} checked files")
                } else {
                    format!("Checked {checked} files | {}", relative.display())
                };
                let total = expected_files.max(checked);
                let _ = worker_sender.send(GuiMessage::SourceProcessingProgress(
                    SourceProcessingProgress {
                        source_id: source_id.clone(),
                        lifecycle_generation,
                        active: true,
                        completed: checked.min(total),
                        total,
                        stage: String::from("Scanning source changes"),
                        detail: source_detail,
                    },
                ));
                last_progress_publish_at = Some(Instant::now());
            };
            let (outcome, incomplete_error) = match audit_source_and_record_with_progress(
                &database,
                Some(cancel),
                MANIFEST_AUDIT_HASH_BATCH,
                completed_at,
                &mut publish_progress,
            ) {
                Ok(outcome) => (outcome, None),
                Err(ScanError::Incomplete { committed, error }) => (*committed, Some(error)),
                Err(error) => return Err(error.to_string()),
            };
            tracing::debug!(
                target: "wavecrate::source_processing",
                source_id = candidate.source.id.as_str(),
                revision = outcome.committed_delta.revision,
                created = outcome.committed_delta.created.len(),
                created_paths = ?outcome
                    .committed_delta
                    .created
                    .iter()
                    .map(|identity| identity.relative_path.as_path())
                    .collect::<Vec<_>>(),
                changed = outcome.committed_delta.changed.len(),
                moved = outcome.committed_delta.moved.len(),
                deleted = outcome.committed_delta.deleted.len(),
                deleted_paths = ?outcome
                    .committed_delta
                    .deleted
                    .iter()
                    .map(|identity| identity.relative_path.as_path())
                    .collect::<Vec<_>>(),
                "Periodic source manifest audit committed"
            );
            if !outcome.committed_delta.is_empty()
                && let Some(worker_sender) = worker_sender
            {
                let _ = worker_sender.send(GuiMessage::SourceManifestAuditCommitted {
                    source_id: candidate.source.id.as_str().to_string(),
                    committed_delta: outcome.committed_delta,
                });
            }
            if cancel.load(Ordering::Acquire) {
                Ok(ExecutionOutcome::Cancelled)
            } else if let Some(error) = incomplete_error {
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    source_id = candidate.source.id.as_str(),
                    error,
                    "Manifest audit published a committed checkpoint and remains due"
                );
                Ok(ExecutionOutcome::Failed)
            } else {
                Ok(ExecutionOutcome::Completed)
            }
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
    Complete(Option<std::path::PathBuf>),
    Retry(&'static str),
    Permanent(&'static str),
    Unsupported(&'static str),
    PrerequisiteInvalidated,
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
        cleanup_unpublished_readiness_output(&outcome);
        return Ok(ExecutionOutcome::Stale);
    }
    if cancel.load(Ordering::Acquire) {
        cleanup_unpublished_readiness_output(&outcome);
        return cancel_claim(
            &mut connection,
            &claim,
            "runtime cancellation before readiness publication",
            now_epoch_seconds(),
        );
    }
    match outcome {
        Ok(ReadinessExecutionOutcome::Complete(artifact_ref)) => {
            let completed = match artifact_ref.as_deref() {
                Some(artifact_ref) => complete_readiness_work_with_artifact_ref(
                    &mut connection,
                    &claim,
                    now_epoch_seconds(),
                    &artifact_ref.to_string_lossy(),
                ),
                None => complete_readiness_work(&mut connection, &claim, now_epoch_seconds()),
            };
            let completed = match completed {
                Ok(completed) => completed,
                Err(error) => {
                    if let Some(artifact_ref) = artifact_ref.as_deref() {
                        invalidate_persisted_waveform_cache_ref(artifact_ref);
                    }
                    return Err(error.to_string());
                }
            };
            match completed {
                ArtifactPublishOutcome::Recorded => Ok(ExecutionOutcome::Completed),
                ArtifactPublishOutcome::RejectedStale => {
                    if let Some(artifact_ref) = artifact_ref.as_deref() {
                        invalidate_persisted_waveform_cache_ref(artifact_ref);
                    }
                    Ok(ExecutionOutcome::Stale)
                }
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
            let classification = readiness_failure_classification(&reason);
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, READINESS_MAX_ATTEMPTS)
                .expect("valid readiness retry policy");
            let outcome = fail_readiness_work(
                &mut connection,
                &claim,
                classification,
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
        Ok(ReadinessExecutionOutcome::Unsupported(reason)) => {
            let policy =
                ReadinessRetryPolicy::new(5, 5 * 60, 1).expect("valid readiness terminal policy");
            let outcome = fail_readiness_work(
                &mut connection,
                &claim,
                ReadinessFailureClassification::Unsupported,
                reason,
                now_epoch_seconds(),
                policy,
            )
            .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
        Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated) => {
            match release_readiness_work(&mut connection, &claim, now_epoch_seconds())
                .map_err(|error| error.to_string())?
            {
                ReadinessWorkMutationOutcome::Recorded => {
                    Ok(ExecutionOutcome::PrerequisiteInvalidated)
                }
                ReadinessWorkMutationOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
            }
        }
    }
}

fn cleanup_unpublished_readiness_output(outcome: &Result<ReadinessExecutionOutcome, String>) {
    if let Ok(ReadinessExecutionOutcome::Complete(Some(artifact_ref))) = outcome {
        invalidate_persisted_waveform_cache_ref(artifact_ref);
    }
}

fn readiness_failure_classification(reason: &str) -> ReadinessFailureClassification {
    if is_known_unsupported_audio_failure(reason) {
        ReadinessFailureClassification::Unsupported
    } else {
        ReadinessFailureClassification::Retryable
    }
}

fn is_known_unsupported_audio_failure(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("failed to decode audio file:")
        || reason.contains("audio decode failed for")
        || reason.contains("audio file contains no complete frames")
        || reason.contains("unsupported codec")
        || reason.contains("no suitable format reader found")
}

fn reclassify_known_unsupported_audio_failures(
    connection: &mut rusqlite::Connection,
) -> Result<usize, String> {
    let legacy_terminal_failures = {
        let mut statement = connection
            .prepare(
                "SELECT id, COALESCE(last_error, '')
                 FROM analysis_jobs
                 WHERE readiness_managed = 1
                   AND status = 'failed'
                   AND failure_kind IN ('retryable', 'permanent')",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    let unsupported_ids = legacy_terminal_failures
        .into_iter()
        .filter_map(|(id, error)| is_known_unsupported_audio_failure(&error).then_some(id))
        .collect::<Vec<_>>();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let mut reclassified = 0_usize;
    for id in unsupported_ids {
        reclassified = reclassified.saturating_add(
            transaction
                .execute(
                    "UPDATE analysis_jobs
                     SET failure_kind = 'unsupported', retry_at = NULL
                     WHERE id = ?1
                       AND readiness_managed = 1
                       AND status = 'failed'
                       AND failure_kind IN ('retryable', 'permanent')",
                    [id],
                )
                .map_err(|error| error.to_string())?,
        );
    }
    reclassified = reclassified.saturating_add(
        transaction
            .execute(
                "UPDATE analysis_jobs AS dependent
                 SET failure_kind = 'unsupported', retry_at = NULL
                 WHERE dependent.readiness_managed = 1
                   AND dependent.status = 'failed'
                   AND dependent.failure_kind IN ('retryable', 'permanent')
                   AND dependent.readiness_stage = 'embedding_aspects'
                   AND dependent.last_error =
                       'embedding feature prerequisite is not durable yet'
                   AND EXISTS(
                       SELECT 1
                       FROM analysis_jobs AS prerequisite
                       WHERE prerequisite.readiness_managed = 1
                         AND prerequisite.source_id = dependent.source_id
                         AND prerequisite.readiness_scope_kind =
                             dependent.readiness_scope_kind
                         AND prerequisite.readiness_scope_id =
                             dependent.readiness_scope_id
                         AND prerequisite.readiness_stage = 'analysis_features'
                         AND prerequisite.content_generation =
                             dependent.content_generation
                         AND prerequisite.status = 'failed'
                         AND prerequisite.failure_kind = 'unsupported'
                   )",
                [],
            )
            .map_err(|error| error.to_string())?,
    );
    reclassified = reclassified.saturating_add(
        transaction
            .execute(
                "UPDATE analysis_jobs
                 SET failure_kind = 'retryable',
                     attempts = 0,
                     retry_at = NULL
                 WHERE readiness_managed = 1
                   AND status = 'failed'
                   AND failure_kind = 'permanent'
                   AND readiness_stage = 'embedding_aspects'
                   AND last_error =
                       'embedding feature prerequisite is not durable yet'",
                [],
            )
            .map_err(|error| error.to_string())?,
    );
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(reclassified)
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
                let committed_content_hash = connection
                    .query_row(
                        "SELECT content_hash FROM wav_files
                         WHERE file_identity = ?1 AND path = ?2 AND missing = 0",
                        params![target.scope_id, relative_path],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .flatten()
                    .filter(|content_hash| !content_hash.is_empty());
                if committed_content_hash.as_deref() == Some(target.content_generation.as_str()) {
                    ReadinessExecutionOutcome::Complete(None)
                } else if committed_content_hash.is_some() {
                    ReadinessExecutionOutcome::PrerequisiteInvalidated
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
            if target.content_generation.starts_with("pending-") {
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated);
            }
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "playback summary target has no relative path",
                ));
            };
            let absolute_path = source.root.join(relative_path);
            // A claimed deficit has no exact current cache owner. Do not hydrate an unowned v4
            // payload using only size/mtime; erase that key first so this generation produces a
            // cache that can be committed atomically with its durable v5 ownership reference.
            if !prepare_playback_cache_generation(connection, target, &absolute_path)? {
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated);
            }
            let cache_ref = ensure_persisted_playback_summary(absolute_path, cancel)?;
            Ok(ReadinessExecutionOutcome::Complete(Some(cache_ref)))
        }
        ReadinessStage::AnalysisFeatures => {
            if target.content_generation.starts_with("pending-") {
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated);
            }
            let Some(relative_path) = target.relative_path.as_deref() else {
                return Ok(ReadinessExecutionOutcome::Permanent(
                    "analysis feature target has no relative path",
                ));
            };
            if readiness_stage_is_unsupported(connection, target, "playback_summary")? {
                return Ok(ReadinessExecutionOutcome::Unsupported(
                    "playback prerequisite is unsupported for this content generation",
                ));
            }
            if target.required_version != wavecrate_analysis::analysis_version() {
                return Ok(ReadinessExecutionOutcome::Retry(
                    "feature executor version does not match target",
                ));
            }
            if analysis_features_are_current(connection, target)? {
                return Ok(ReadinessExecutionOutcome::Complete(None));
            }
            let produced = super::worker::run_readiness_feature_stage(
                connection,
                source,
                std::path::Path::new(relative_path),
                target.content_generation.as_str(),
                target.required_version.as_str(),
                cancel,
            )?;
            if produced && analysis_features_are_current(connection, target)? {
                return Ok(ReadinessExecutionOutcome::Complete(None));
            }
            if !produced {
                reconcile_stale_analysis_input(
                    source,
                    std::path::Path::new(relative_path),
                    cancel,
                )?;
                return Ok(ReadinessExecutionOutcome::Retry(
                    "analysis input changed; targeted source reconciliation committed",
                ));
            }
            Ok(ReadinessExecutionOutcome::Retry(
                "analysis feature publication is not durable yet",
            ))
        }
        ReadinessStage::EmbeddingAspects => {
            if target.content_generation.starts_with("pending-") {
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated);
            }
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
            if readiness_stage_is_unsupported(connection, target, "analysis_features")? {
                return Ok(ReadinessExecutionOutcome::Unsupported(
                    "analysis prerequisite is unsupported for this content generation",
                ));
            }
            let mut analysis_target = target.clone();
            analysis_target.stage = ReadinessStage::AnalysisFeatures;
            analysis_target.required_version = wavecrate_analysis::analysis_version().to_string();
            if !analysis_features_are_current(connection, &analysis_target)? {
                if invalidate_readiness_artifact(connection, &analysis_target)
                    .map_err(|error| error.to_string())?
                {
                    tracing::warn!(
                        target: "wavecrate::source_processing",
                        source_id = target.source_id,
                        scope_id = target.scope_id,
                        "Invalidated an analysis readiness marker whose payload was missing"
                    );
                }
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated);
            }
            Ok(if embedding_aspects_are_current(connection, target)? {
                ReadinessExecutionOutcome::Complete(None)
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
                    ReadinessExecutionOutcome::Complete(None)
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "embedding feature prerequisite is not durable yet",
                    )
                }
            })
        }
        ReadinessStage::SimilarityLayout => {
            if target.required_version != native_similarity_artifact_version() {
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
            finalize_similarity_artifacts_if_ready(source, &publication_fence, cancel).map(
                |finalized| {
                    if finalized {
                        ReadinessExecutionOutcome::Complete(None)
                    } else {
                        ReadinessExecutionOutcome::PrerequisiteInvalidated
                    }
                },
            )
        }
    }
}

fn prepare_playback_cache_generation(
    connection: &mut rusqlite::Connection,
    target: &ReadinessTarget,
    absolute_path: &std::path::Path,
) -> Result<bool, String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let current = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM source_readiness_sources AS source
                JOIN source_readiness_targets AS target
                  ON target.source_id = source.source_id
                 AND target.source_generation = source.source_generation
                WHERE target.source_id = ?1
                  AND target.scope_kind = 'file'
                  AND target.scope_id = ?2
                  AND target.relative_path = ?3
                  AND target.stage = 'playback_summary'
                  AND target.required_version = ?4
                  AND target.content_generation = ?5
                  AND target.eligibility = 'eligible'
                  AND source.availability = 'active'
            )",
            params![
                target.source_id,
                target.scope_id,
                target.relative_path,
                target.required_version,
                target.content_generation,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| error.to_string())?;
    if !current {
        transaction.rollback().map_err(|error| error.to_string())?;
        return Ok(false);
    }
    invalidate_persisted_waveform_cache_path(absolute_path);
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(true)
}

fn readiness_stage_is_unsupported(
    connection: &rusqlite::Connection,
    target: &ReadinessTarget,
    stage: &str,
) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM analysis_jobs
                WHERE readiness_managed = 1
                  AND source_id = ?1
                  AND readiness_scope_kind = 'file'
                  AND readiness_scope_id = ?2
                  AND readiness_stage = ?3
                  AND content_generation = ?4
                  AND status = 'failed'
                  AND failure_kind = 'unsupported'
            )",
            params![
                target.source_id,
                target.scope_id,
                stage,
                target.content_generation
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn reconcile_stale_analysis_input(
    source: &SampleSource,
    relative_path: &std::path::Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let db =
        SourceDatabase::open_for_background_job_with_database_root(&source.root, database_root)
            .map_err(|error| error.to_string())?;
    let stats = sync_paths_with_progress(
        &db,
        &[relative_path.to_path_buf()],
        Some(cancel),
        &mut |_, _| {},
    )
    .map_err(|error| error.to_string())?;
    tracing::info!(
        target: "wavecrate::source_processing",
        source_id = source.id.as_str(),
        path = %relative_path.display(),
        revision = stats.committed_delta.revision,
        changed = stats.committed_delta.changed.len(),
        "Reconciled stale analysis input against the source manifest"
    );
    Ok(())
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

fn recovered_source_retirements(
    active_sources: &BTreeMap<String, SampleSource>,
    next_lifecycle_generation: &mut u64,
) -> Result<(BTreeMap<u64, PendingSourceRetirement>, u64), String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    let mut pending = BTreeMap::new();
    let mut next_retirement_id = 1_u64;
    for retained in retained_sources.into_iter().filter(|retained| {
        !active_sources
            .values()
            .any(|active| source_storage_identity_matches(active, retained))
    }) {
        let lifecycle_generation = *next_lifecycle_generation;
        *next_lifecycle_generation = (*next_lifecycle_generation).wrapping_add(1).max(1);
        pending.insert(
            next_retirement_id,
            PendingSourceRetirement {
                source: retained,
                lifecycle_generation,
                retry_at: 0,
            },
        );
        next_retirement_id = next_retirement_id.wrapping_add(1).max(1);
    }
    Ok((pending, next_retirement_id))
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

fn source_storage_identity_matches(left: &SampleSource, right: &SampleSource) -> bool {
    if left.id != right.id {
        return false;
    }
    match (left.database_root(), right.database_root()) {
        (Ok(left_root), Ok(right_root)) => left_root == right_root,
        _ => false,
    }
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
            aggregate.progress_completed = aggregate
                .progress_completed
                .saturating_add(source.progress_completed);
            aggregate.progress_total = aggregate
                .progress_total
                .saturating_add(source.progress_total);
            aggregate
        })
}

fn coordinator_wait_duration(
    next_retry_at: Option<i64>,
    now: i64,
    safety_wait: Duration,
    processing_paused: bool,
) -> Duration {
    if processing_paused {
        return safety_wait;
    }
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
    use std::path::{Path, PathBuf};

    use crate::native_app::waveform::cached_waveform_file_audition_ready_exists;

    use wavecrate::sample_sources::{
        SourceId,
        readiness::{
            ReadinessArtifact, ReadinessEligibility, SourceAvailability,
            persist_readiness_deficits, publish_readiness_artifact, readiness_work_stats,
            reconcile_readiness, replace_readiness_targets,
        },
    };

    use super::*;

    #[test]
    fn retired_jobs_are_pruned_and_only_readiness_jobs_are_recovered() {
        let directory = tempfile::tempdir().expect("source directory");
        let source = SampleSource::new_with_id(
            SourceId::from_string("current-only-jobs"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create source database");
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        connection
            .execute_batch(
                "INSERT INTO analysis_jobs
                    (sample_id, source_id, relative_path, job_type, status, created_at,
                     readiness_managed)
                 VALUES
                    ('legacy::sample', 'current-only-jobs', 'legacy.wav',
                     'wav_metadata_v1', 'running', 1, 0),
                    ('current::sample', 'current-only-jobs', 'current.wav',
                     'wav_metadata_v1', 'running', 1, 1);
                 INSERT INTO analysis_job_progress_snapshots
                    (job_type, pending, running, done, failed)
                 VALUES ('wav_metadata_v1', 0, 1, 0, 0);",
            )
            .expect("seed current and retired jobs");

        let reset =
            reset_interrupted_readiness_jobs(&source).expect("reset interrupted current job");
        assert_eq!(reset, 1);
        assert_eq!(
            connection
                .query_row(
                    "SELECT status FROM analysis_jobs WHERE sample_id = 'legacy::sample'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("read retired job"),
            "running"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT status FROM analysis_jobs WHERE sample_id = 'current::sample'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("read readiness job"),
            "pending"
        );

        assert_eq!(
            prune_legacy_similarity_jobs(&connection).expect("prune retired jobs"),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM analysis_jobs WHERE readiness_managed = 0",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count retired jobs"),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM analysis_jobs WHERE readiness_managed = 1",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count readiness jobs"),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM analysis_job_progress_snapshots",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count retired progress"),
            0
        );
    }

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
    fn playback_pause_reports_transition_and_fences_source_processing_feedback() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut supervisor = SourceProcessingSupervisor::start_with_playback_state_and_sender(
            Vec::new(),
            false,
            Some(sender),
        );

        supervisor.set_playback_active(true);

        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("playback pause cleanup");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(progress.active);
        assert_eq!(progress.stage, "Pausing source processing");
        assert_eq!(progress.detail, "Waiting for playback");
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("playback pause acknowledgement");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(!progress.active);

        supervisor.set_playback_active(false);
        supervisor.set_foreground_activity(true);
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("foreground activity cleanup");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(progress.active);
        assert_eq!(progress.stage, "Pausing source processing");
        assert_eq!(progress.detail, "Waiting for source loading");
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("foreground activity pause acknowledgement");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(!progress.active);

        supervisor.set_foreground_activity(false);
        supervisor.set_foreground_activity(true);
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("rapid foreground pause transition");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(progress.active);
        supervisor.set_foreground_activity(false);
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("rapid foreground pause acknowledgement");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert!(!progress.active);

        supervisor.set_playback_active(true);
        for expectation in ["final pause transition", "final pause acknowledgement"] {
            let message = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect(expectation);
            let GuiMessage::SourceProcessingProgress(progress) = message else {
                panic!("unexpected supervisor GUI message: {message:?}");
            };
            if expectation == "final pause transition" {
                assert!(progress.active);
            } else {
                assert!(!progress.active);
            }
        }

        let directory = tempfile::tempdir().expect("paused discovery source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("paused-discovery"),
            directory.path().to_path_buf(),
        );
        publish_source_processing_discovery(&supervisor.shared, &source);
        assert!(receiver.try_recv().is_err());
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn repeated_discovery_keeps_existing_processing_feedback_stable() {
        let source = SampleSource::new_with_id(
            SourceId::from_string("stable-discovery"),
            PathBuf::from("/library/samples"),
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source.clone()], Some(sender));

        assert!(publish_source_processing_discovery_if_needed(
            &shared,
            std::slice::from_ref(&source),
            false,
        ));
        let GuiMessage::SourceProcessingProgress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("initial discovery feedback")
        else {
            panic!("unexpected supervisor GUI message");
        };
        assert_eq!(progress.stage, "Checking pending work");

        assert!(!publish_source_processing_discovery_if_needed(
            &shared,
            &[source],
            true,
        ));
        assert!(
            receiver.try_recv().is_err(),
            "an immediate rediscovery must not overwrite visible execution progress"
        );
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
    fn failed_async_retirement_retries_without_reactivating_removed_source() {
        let (_directory, source) = unhashed_source("retirement-fence-failure");
        let database_path = source.db_path().expect("source database path");
        std::fs::remove_file(&database_path).expect("remove source database");
        std::fs::create_dir(&database_path).expect("replace database with invalid directory");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");

        supervisor
            .replace_sources(Vec::new())
            .expect("removal returns before asynchronous cleanup");
        process_ready_source_retirements(&supervisor.shared);
        supervisor.wake_source(source.id.as_str(), "late_watcher_event");
        supervisor.set_playback_active(true);
        supervisor.set_playback_active(false);

        let control = supervisor.shared.control();
        assert!(!control.sources.contains_key(source.id.as_str()));
        assert!(!control.dirty_sources.contains(source.id.as_str()));
        assert_eq!(control.pending_retirements.len(), 1);
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
            .shared
            .control()
            .pending_retirements
            .values_mut()
            .for_each(|retirement| retirement.retry_at = 0);
        process_ready_source_retirements(&supervisor.shared);
        assert!(supervisor.shared.control().pending_retirements.is_empty());
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
    fn source_removal_returns_immediately_and_retires_after_exact_epoch_drains() {
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
        let started = Instant::now();
        supervisor
            .replace_sources(Vec::new())
            .expect("remove configured source");
        assert!(started.elapsed() < Duration::from_millis(50));
        process_ready_source_retirements(&supervisor.shared);
        let active_before_drain: String =
            SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            )
            .expect("reopen readiness before old work drains")
            .query_row(
                "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read readiness before old work drains");
        assert_eq!(active_before_drain, "active");
        drop(in_flight);
        process_ready_source_retirements(&supervisor.shared);

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
    fn fast_readd_waits_for_old_epoch_then_preserves_shared_source_storage() {
        let (_directory, source) = unhashed_source("fast-readd-retains-storage");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 1)
            .expect("publish readiness targets");
        drop(connection);

        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let old_cancel =
            Arc::clone(&supervisor.shared.control().source_work_cancels[source.id.as_str()]);
        let old_work = supervisor
            .shared
            .begin_in_flight_work(source.id.as_str(), &old_cancel)
            .expect("register old epoch work");

        supervisor
            .replace_sources(Vec::new())
            .expect("remove source immediately");
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("re-add source immediately");
        assert!(
            supervisor
                .budget_handle()
                .acquire_scan(source.id.as_str())
                .is_none(),
            "same-storage admission stays blocked while the retired epoch is still active"
        );
        process_ready_source_retirements(&supervisor.shared);
        assert_eq!(supervisor.shared.control().pending_retirements.len(), 1);

        drop(old_work);
        process_ready_source_retirements(&supervisor.shared);
        assert!(supervisor.shared.control().pending_retirements.is_empty());
        assert!(
            supervisor
                .budget_handle()
                .acquire_scan(source.id.as_str())
                .is_some()
        );
        let availability: String = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen retained source database")
        .query_row(
            "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
            [source.id.as_str()],
            |row| row.get(0),
        )
        .expect("read retained source readiness");
        assert_eq!(availability, "active");
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn startup_recovery_enqueues_retained_sources_missing_from_configuration() {
        let config_base = tempfile::tempdir().expect("config base");
        let _guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
        let source_root = tempfile::tempdir().expect("retained source root");
        let source = SampleSource::new_with_id(
            SourceId::from_string("startup-retirement"),
            source_root.path().to_path_buf(),
        );
        wavecrate::sample_sources::library::save(
            &wavecrate::sample_sources::library::LibraryState {
                sources: vec![source.clone()],
            },
        )
        .expect("remember retained source");
        wavecrate::sample_sources::library::save(
            &wavecrate::sample_sources::library::LibraryState::default(),
        )
        .expect("remove active configuration while retaining descriptor");

        let mut next_generation = 1;
        let (pending, next_retirement_id) =
            recovered_source_retirements(&BTreeMap::new(), &mut next_generation)
                .expect("recover inactive retained source");

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[&1].source.id, source.id);
        assert_eq!(next_retirement_id, 2);
        assert_eq!(next_generation, 2);
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
    fn foreground_scan_admission_preempts_incompatible_background_work() {
        let (_first_directory, first) = unhashed_source("background-holder");
        let (_second_directory, second) = unhashed_source("foreground-waiter");
        let shared = Arc::new(Shared::new(vec![first.clone(), second.clone()], None));
        let background_cancel = {
            let control = shared.control();
            Arc::clone(&control.source_work_cancels[first.id.as_str()])
        };
        let background_permit = shared
            .budgets()
            .try_acquire(first.id.as_str(), ProcessingLane::Hashing)
            .expect("reserve database capacity for background hashing");
        let waiting_shared = Arc::clone(&shared);
        let foreground_source_id = second.id.to_string();
        let waiting = thread::spawn(move || {
            SourceProcessingBudgetHandle {
                shared: waiting_shared,
            }
            .acquire_scan(&foreground_source_id)
        });

        wait_until(Duration::from_secs(2), || {
            background_cancel.load(Ordering::Acquire)
                && shared.external_scans().admissions.len() == 1
        });
        shared.budgets().release(background_permit);
        shared.budget_wake.notify_all();

        let foreground_permit = waiting
            .join()
            .expect("join foreground admission")
            .expect("foreground scan acquires released lane");
        assert_eq!(
            foreground_permit
                .permit
                .as_ref()
                .expect("owned budget permit")
                .source_id(),
            second.id.as_str()
        );
        drop(foreground_permit);
    }

    #[test]
    fn foreground_scan_admission_reserves_all_processing_capacity() {
        let (_directory, source) = unhashed_source("foreground-reservation");
        let candidates = vec![
            RuntimeCandidate {
                schedule: WorkCandidate::source(source.id.as_str(), ProcessingLane::Scan, 0, 0),
                source: source.clone(),
                task: RuntimeTask::ManifestAudit,
            },
            RuntimeCandidate {
                schedule: WorkCandidate::source(source.id.as_str(), ProcessingLane::Hashing, 0, 0),
                source,
                task: RuntimeTask::ManifestAudit,
            },
        ];

        assert_eq!(scheduler_candidate_indices(&candidates, false), vec![0, 1]);
        assert!(scheduler_candidate_indices(&candidates, true).is_empty());
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
    fn manifest_audit_is_scheduled_only_when_the_active_source_is_due() {
        let directory = tempfile::tempdir().expect("manifest audit source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("manifest-audit"),
            directory.path().to_path_buf(),
        );
        let db = source.open_db().expect("open manifest audit source");
        let cancel = AtomicBool::new(false);

        let Cancellable::Completed((due, _)) =
            discover_source_candidates(&source, MANIFEST_AUDIT_INTERVAL_SECONDS, &cancel)
                .expect("discover due manifest audit")
        else {
            panic!("manifest audit discovery unexpectedly cancelled");
        };
        assert!(
            due.iter()
                .any(|candidate| matches!(candidate.task, RuntimeTask::ManifestAudit))
        );

        db.set_metadata(
            META_LAST_MANIFEST_AUDIT_AT,
            &MANIFEST_AUDIT_INTERVAL_SECONDS.to_string(),
        )
        .expect("record manifest audit");
        let Cancellable::Completed((not_due, _)) =
            discover_source_candidates(&source, MANIFEST_AUDIT_INTERVAL_SECONDS * 2 - 1, &cancel)
                .expect("discover recent manifest audit")
        else {
            panic!("manifest audit discovery unexpectedly cancelled");
        };
        assert!(
            not_due
                .iter()
                .all(|candidate| !matches!(candidate.task, RuntimeTask::ManifestAudit))
        );
    }

    #[test]
    fn missing_manifest_identity_schedules_self_healing_audit_even_when_recent() {
        let (_directory, source) = unhashed_source("manifest-identity-repair");
        let db = source
            .open_db()
            .expect("open manifest identity repair source");
        let mut batch = db.write_batch().expect("open missing identity batch");
        batch
            .set_file_identity(Path::new("pending.wav"), None)
            .expect("clear manifest identity");
        batch.commit().expect("commit missing manifest identity");
        db.set_metadata(META_LAST_MANIFEST_AUDIT_AT, "100")
            .expect("record recent audit");
        let cancel = AtomicBool::new(false);

        let Cancellable::Completed((candidates, _)) =
            discover_source_candidates(&source, 100, &cancel)
                .expect("discover manifest identity repair")
        else {
            panic!("manifest identity repair discovery unexpectedly cancelled");
        };

        assert!(
            candidates
                .iter()
                .any(|candidate| matches!(candidate.task, RuntimeTask::ManifestAudit))
        );
    }

    #[test]
    fn appledouble_sidecars_do_not_keep_manifest_audits_permanently_due() {
        let directory = tempfile::tempdir().expect("AppleDouble source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("appledouble-audit"),
            directory.path().to_path_buf(),
        );
        let db = source.open_db().expect("open AppleDouble source");
        db.upsert_file(Path::new("folder/._sidecar.wav"), 4_096, 1)
            .expect("seed legacy AppleDouble row");
        db.set_metadata(META_LAST_MANIFEST_AUDIT_AT, "100")
            .expect("record recent audit");
        let cancel = AtomicBool::new(false);

        let Cancellable::Completed((candidates, _)) =
            discover_source_candidates(&source, 100, &cancel)
                .expect("discover source with ignored AppleDouble row")
        else {
            panic!("AppleDouble source discovery unexpectedly cancelled");
        };

        assert!(
            candidates
                .iter()
                .all(|candidate| !matches!(candidate.task, RuntimeTask::ManifestAudit))
        );
    }

    #[test]
    fn missing_source_discovery_updates_external_metadata_without_recreating_audio_root() {
        let parent = tempfile::tempdir().expect("missing source parent");
        let root = parent.path().join("source");
        std::fs::create_dir(&root).expect("create source root");
        let source =
            SampleSource::new_with_id(SourceId::from_string("missing-source"), root.clone())
                .protected();
        let database_root = source.database_root().expect("external metadata root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("create external source database");
        connection
            .execute(
                "INSERT INTO source_readiness_sources (
                    source_id, source_generation, readiness_revision, availability, updated_at
                 ) VALUES (?1, 1, 1, 'active', 1)",
                [source.id.as_str()],
            )
            .expect("publish active source readiness");
        drop(connection);
        std::fs::remove_dir_all(&root).expect("remove source root");
        let cancel = AtomicBool::new(false);

        let Cancellable::Completed((candidates, _)) =
            discover_source_candidates(&source, 100, &cancel).expect("discover unavailable source")
        else {
            panic!("missing source discovery unexpectedly cancelled");
        };

        assert!(candidates.is_empty());
        assert!(
            !root.exists(),
            "discovery must not recreate a missing source"
        );
        let connection = SourceDatabase::open_unavailable_source_metadata_connection(
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen external source metadata");
        let availability: String = connection
            .query_row(
                "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read missing source availability");
        assert_eq!(availability, "offline");
    }

    #[test]
    fn scheduled_manifest_audit_does_not_recreate_source_removed_after_discovery() {
        let parent = tempfile::tempdir().expect("missing source parent");
        let root = parent.path().join("source");
        std::fs::create_dir(&root).expect("create source root");
        let source = SampleSource::new_with_id(
            SourceId::from_string("removed-after-discovery"),
            root.clone(),
        );
        source.open_db().expect("create source database");
        let candidate = RuntimeCandidate {
            schedule: WorkCandidate::source(
                source.id.as_str(),
                ProcessingLane::Scan,
                0,
                now_epoch_seconds(),
            ),
            source,
            task: RuntimeTask::ManifestAudit,
        };
        std::fs::remove_dir_all(&root).expect("remove source after scheduling");

        assert_eq!(
            execute_candidate(&candidate, 0, &AtomicBool::new(false), None)
                .expect("unavailable audit is parked"),
            ExecutionOutcome::Parked
        );
        assert!(
            !should_requeue_cancelled(Some(ExecutionOutcome::Parked), true, false),
            "unavailable roots must wait for a later availability or safety wake"
        );
        assert!(
            !root.exists(),
            "executing stale scheduled work must not recreate the source"
        );
    }

    #[test]
    fn readiness_progress_publishes_determinate_source_job_feedback() {
        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("progress-source"),
            directory.path().to_path_buf(),
        );
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "identity-1",
            "drums/kick.wav",
            ReadinessStage::EmbeddingAspects,
            "embedding-v1",
            1,
            "content-1",
        );
        let candidate = RuntimeCandidate {
            schedule: WorkCandidate::readiness(&target, 1),
            source: source.clone(),
            task: RuntimeTask::Readiness(target),
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source], Some(sender));

        publish_source_processing_progress(
            &shared,
            &candidate,
            SourceDiscoveryStats {
                progress_completed: 313,
                progress_total: 9_985,
                ..SourceDiscoveryStats::default()
            },
        );

        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("progress message");
        let GuiMessage::SourceProcessingProgress(progress) = message else {
            panic!("unexpected supervisor GUI message: {message:?}");
        };
        assert_eq!(progress.source_id, "progress-source");
        assert!(progress.active);
        assert_eq!(progress.completed, 313);
        assert_eq!(progress.total, 9_985);
        assert_eq!(progress.stage, "Preparing similarity");
        assert_eq!(progress.detail, "drums/kick.wav");
    }

    #[test]
    fn executing_candidate_remains_active_at_discovery_counter_boundary() {
        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("boundary-source"),
            directory.path().to_path_buf(),
        );
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "identity-1",
            "drums/kick.wav",
            ReadinessStage::AnalysisFeatures,
            "analysis-v1",
            1,
            "content-1",
        );
        let candidate = RuntimeCandidate {
            schedule: WorkCandidate::readiness(&target, 1),
            source: source.clone(),
            task: RuntimeTask::Readiness(target),
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source], Some(sender));

        publish_source_processing_progress(
            &shared,
            &candidate,
            SourceDiscoveryStats {
                progress_completed: 25_000,
                progress_total: 25_000,
                ..SourceDiscoveryStats::default()
            },
        );

        let GuiMessage::SourceProcessingProgress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("progress message")
        else {
            panic!("unexpected supervisor GUI message");
        };
        assert!(progress.active);
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.total, 0);
        assert_eq!(progress.stage, "Analyzing audio");
    }

    #[test]
    fn readiness_progress_counts_remain_scoped_to_the_reported_source() {
        let mut source_stats = BTreeMap::from([
            (
                String::from("first"),
                SourceDiscoveryStats {
                    progress_completed: 25_000,
                    progress_total: 26_000,
                    ..SourceDiscoveryStats::default()
                },
            ),
            (
                String::from("second"),
                SourceDiscoveryStats {
                    progress_completed: 24_000,
                    progress_total: 25_000,
                    ..SourceDiscoveryStats::default()
                },
            ),
        ]);

        let progress = advance_source_progress(&mut source_stats, "first")
            .expect("first source progress advances");

        assert_eq!(progress.progress_completed, 25_001);
        assert_eq!(progress.progress_total, 26_000);
        assert_eq!(source_stats["second"].progress_completed, 24_000);
        assert_eq!(source_stats["second"].progress_total, 25_000);
    }

    #[test]
    fn visible_source_progress_does_not_regress_across_rediscovery() {
        let mut displayed = BTreeMap::new();
        let first = stable_source_progress(
            &mut displayed,
            "projects",
            SourceDiscoveryStats {
                progress_completed: 15_888,
                progress_total: 19_969,
                ..SourceDiscoveryStats::default()
            },
        );
        let rediscovered = stable_source_progress(
            &mut displayed,
            "projects",
            SourceDiscoveryStats {
                progress_completed: 15_747,
                progress_total: 19_969,
                ..SourceDiscoveryStats::default()
            },
        );

        assert_eq!(first.progress_completed, 15_888);
        assert_eq!(rediscovered.progress_completed, 15_888);
        assert_eq!(rediscovered.progress_total, 19_969);
    }

    #[test]
    fn visible_source_progress_starts_a_coherent_snapshot_when_total_changes() {
        let mut displayed = BTreeMap::new();
        let inflated = stable_source_progress(
            &mut displayed,
            "projects",
            SourceDiscoveryStats {
                progress_completed: 44_029,
                progress_total: 46_678,
                ..SourceDiscoveryStats::default()
            },
        );
        let current = stable_source_progress(
            &mut displayed,
            "projects",
            SourceDiscoveryStats {
                progress_completed: 9_766,
                progress_total: 9_985,
                ..SourceDiscoveryStats::default()
            },
        );

        assert_eq!(inflated.progress_completed, 44_029);
        assert_eq!(inflated.progress_total, 46_678);
        assert_eq!(current.progress_completed, 9_766);
        assert_eq!(current.progress_total, 9_985);
        assert!(current.progress_completed <= current.progress_total);
    }

    #[test]
    fn periodic_manifest_audit_wakes_browser_projection_after_committed_repair() {
        let directory = tempfile::tempdir().expect("manifest audit source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("audit-browser-wake"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create source database");
        std::fs::write(directory.path().join("missed.wav"), [7_u8; 32])
            .expect("write missed watcher file");
        let (sender, receiver) = std::sync::mpsc::channel();

        let candidate = RuntimeCandidate {
            schedule: WorkCandidate::source(
                source.id.as_str(),
                ProcessingLane::Scan,
                0,
                now_epoch_seconds(),
            ),
            source,
            task: RuntimeTask::ManifestAudit,
        };
        assert_eq!(
            execute_candidate(&candidate, 0, &AtomicBool::new(false), Some(&sender))
                .expect("execute manifest audit"),
            ExecutionOutcome::Completed
        );
        let messages = receiver.try_iter().collect::<Vec<_>>();
        let progress = messages
            .iter()
            .find_map(|message| match message {
                GuiMessage::SourceProcessingProgress(progress) => Some(progress),
                _ => None,
            })
            .expect("audit should publish checked-file progress");
        let (source_id, committed_delta) = messages
            .iter()
            .find_map(|message| match message {
                GuiMessage::SourceManifestAuditCommitted {
                    source_id,
                    committed_delta,
                } => Some((source_id, committed_delta)),
                _ => None,
            })
            .expect("audit should publish a browser projection wake");

        assert_eq!(source_id, "audit-browser-wake");
        assert_eq!(progress.source_id, "audit-browser-wake");
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.total, 1);
        assert_eq!(progress.stage, "Scanning source changes");
        assert!(progress.detail.contains("Checked 1 files"));
        assert_eq!(committed_delta.created.len(), 1);
        assert_eq!(
            committed_delta.created[0].relative_path,
            Path::new("missed.wav")
        );
    }

    #[test]
    fn production_supervisor_publishes_claims_and_completes_readiness_without_manual_seed() {
        let (_directory, source) = ready_analysis_source("readiness");

        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(20), || {
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
                    "SELECT COUNT(*) = 5
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
                          AND artifact.artifact_version = target.required_version
                          AND artifact.content_generation = target.content_generation
                          AND (
                              target.stage != 'playback_summary'
                              OR (
                                  artifact.relative_path = target.relative_path
                                  AND artifact.artifact_ref IS NOT NULL
                                  AND length(trim(artifact.artifact_ref)) > 0
                              )
                          )
                          AND (
                              target.scope_kind = 'file'
                              OR artifact.source_generation = target.source_generation
                          )
                          AND EXISTS (
                              SELECT 1 FROM layout_umap
                              WHERE sample_id = ?1 || '::ready.wav'
                          )
                          AND EXISTS (
                              SELECT 1 FROM hdbscan_clusters
                              WHERE sample_id = ?1 || '::ready.wav'
                          )
                          AND EXISTS (
                              SELECT 1 FROM ann_index_meta WHERE count = 1
                          )
                          AND EXISTS (
                              SELECT 1 FROM metadata
                              WHERE key = 'similarity_artifact_state_v1'
                                AND json_extract(value, '$.state') = 'current'
                                AND json_extract(value, '$.artifact_contract_version') = ?2
                          )",
                    params![source.id.as_str(), native_similarity_artifact_version()],
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
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open converged readiness database");
        let cache_ref = connection
            .query_row(
                "SELECT artifact_ref
                 FROM source_readiness_artifacts
                 WHERE source_id = ?1 AND stage = 'playback_summary'",
                [source.id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .expect("read playback ownership reference");
        assert!(std::path::Path::new(&cache_ref).is_file());
        assert!(
            !std::path::Path::new(&cache_ref)
                .with_extension("pcm")
                .exists(),
            "large-file readiness should remain file-backed without persisted decoded PCM"
        );
    }

    #[test]
    fn committed_delete_retires_exact_owned_playback_cache_without_old_file_metadata() {
        let (_directory, source) = ready_analysis_source("playback-delete");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        let database_root = source.database_root().expect("database root");
        let mut owned_cache_ref = None::<std::path::PathBuf>;
        wait_until(Duration::from_secs(15), || {
            let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            ) else {
                return false;
            };
            owned_cache_ref = connection
                .query_row(
                    "SELECT artifact_ref
                     FROM source_readiness_artifacts
                     WHERE source_id = ?1
                       AND stage = 'playback_summary'
                       AND artifact_ref IS NOT NULL",
                    [source.id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .ok()
                .flatten()
                .map(std::path::PathBuf::from);
            owned_cache_ref.as_ref().is_some_and(|path| path.is_file())
        });
        let owned_cache_ref = owned_cache_ref.expect("owned playback cache ref");

        std::fs::remove_file(source.root.join("ready.wav")).expect("delete source sample");
        let db = source.open_db().expect("open source after delete");
        wavecrate::sample_sources::scanner::sync_paths(&db, &[PathBuf::from("ready.wav")])
            .expect("commit source deletion");
        supervisor.wake_source(source.id.as_str(), "test_committed_delete");

        wait_until(Duration::from_secs(10), || {
            let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            ) else {
                return false;
            };
            let ownership_removed = connection
                .query_row(
                    "SELECT COUNT(*) = 0
                     FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary'",
                    [source.id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap_or(false);
            ownership_removed && !owned_cache_ref.exists()
        });
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
    }

    #[test]
    fn stale_analysis_hash_triggers_targeted_reconciliation_and_converges() {
        let (_directory, source) = ready_analysis_source("stale-analysis-input");
        let relative = Path::new("ready.wav");
        let db = source.open_db().expect("open stale analysis source");
        wavecrate::sample_sources::scanner::sync_paths(&db, &[relative.to_path_buf()])
            .expect("normalize source manifest");
        db.set_metadata(
            META_LAST_MANIFEST_AUDIT_AT,
            &now_epoch_seconds().to_string(),
        )
        .expect("defer periodic audit");

        let path = source.root.join(relative);
        let original_modified = std::fs::metadata(&path)
            .expect("read original metadata")
            .modified()
            .expect("read original modified time");
        let mut bytes = std::fs::read(&path).expect("read readiness wav");
        let last = bytes.last_mut().expect("readiness wav has audio data");
        *last ^= 0x01;
        std::fs::write(&path, &bytes).expect("mutate readiness wav");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("reopen mutated readiness wav");
        file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
            .expect("restore readiness modified time");
        let current_hash = blake3::hash(&bytes).to_hex().to_string();

        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        wait_until(Duration::from_secs(15), || {
            let manifest_is_current = source
                .open_db()
                .ok()
                .and_then(|db| db.entry_for_path(relative).ok().flatten())
                .and_then(|entry| entry.content_hash)
                .as_deref()
                == Some(current_hash.as_str());
            if !manifest_is_current {
                return false;
            }
            let database_root = source.database_root().expect("database root");
            let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            ) else {
                return false;
            };
            let sample_id = format!("{}::ready.wav", source.id);
            connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM samples AS sample
                        JOIN features AS feature ON feature.sample_id = sample.sample_id
                        JOIN embeddings AS embedding ON embedding.sample_id = sample.sample_id
                        JOIN similarity_aspect_descriptors AS aspects
                          ON aspects.sample_id = sample.sample_id
                        WHERE sample.sample_id = ?1
                          AND sample.content_hash = ?2
                    )",
                    params![sample_id, current_hash],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap_or(false)
        });
        assert_eq!(supervisor.shutdown()["joined"], true);
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
            let good_hashed = source
                .open_db()
                .expect("open hash source")
                .entry_for_path(Path::new("z-good.wav"))
                .expect("read good hash row")
                .and_then(|entry| entry.content_hash)
                .is_some();
            let failure_recorded = SourceDatabase::open_connection_with_role_and_database_root(
                &source.root,
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            )
            .ok()
            .and_then(|connection| {
                connection
                    .query_row(
                        "SELECT status
                         FROM analysis_jobs
                         WHERE readiness_managed = 1
                           AND readiness_stage = 'indexed_identity'
                           AND relative_path = 'a-unavailable.wav'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
            })
            .as_deref()
                == Some("failed");
            good_hashed && failure_recorded
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
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Arc::new(Shared::new(vec![source.clone()], Some(sender)));
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
        assert!(
            receiver.try_iter().all(|message| matches!(
                message,
                GuiMessage::SourceProcessingProgress(SourceProcessingProgress {
                    active: false,
                    ..
                })
            )),
            "queued work must not publish active progress while foreground admission owns the lane"
        );

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
        process_ready_source_retirements(&supervisor.shared);
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
    fn zero_byte_audio_is_terminally_non_analyzable_and_never_enters_the_work_queue() {
        let directory = tempfile::tempdir().expect("zero-byte source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("zero-byte-source"),
            directory.path().to_path_buf(),
        );
        let db = source.open_db().expect("open zero-byte source");
        db.upsert_file(Path::new("empty.wav"), 0, 1)
            .expect("insert zero-byte manifest row");
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
                 SET file_identity = 'zero-byte-identity',
                     content_hash = 'zero-byte-content'
                 WHERE path = 'empty.wav'",
                [],
            )
            .expect("assign zero-byte identity");

        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
                .expect("publish zero-byte targets")
        );
        let targets = {
            let mut statement = connection
                .prepare(
                    "SELECT stage, eligibility
                     FROM source_readiness_targets
                     WHERE source_id = ?1 AND scope_id = 'zero-byte-identity'
                     ORDER BY stage",
                )
                .expect("prepare zero-byte targets");
            statement
                .query_map([source.id.as_str()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .expect("query zero-byte targets")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect zero-byte targets")
        };
        assert_eq!(
            targets,
            vec![
                (
                    String::from("analysis_features"),
                    String::from("unsupported")
                ),
                (
                    String::from("embedding_aspects"),
                    String::from("unsupported")
                ),
                (String::from("indexed_identity"), String::from("eligible")),
                (
                    String::from("playback_summary"),
                    String::from("unsupported")
                ),
            ]
        );

        let snapshot = reconcile_readiness(&connection, source.id.as_str(), 100)
            .expect("reconcile zero-byte targets");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, 100)
            .expect("persist zero-byte deficits");
        let queued_non_analyzable: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM analysis_jobs
                 WHERE readiness_scope_id = 'zero-byte-identity'
                   AND readiness_stage IN (
                       'playback_summary', 'analysis_features', 'embedding_aspects'
                   )",
                [],
                |row| row.get(0),
            )
            .expect("count zero-byte readiness work");
        assert_eq!(queued_non_analyzable, 0);
    }

    #[test]
    fn deferred_full_hash_blocks_all_content_derived_targets_until_identity_is_exact() {
        let (_directory, source) = unhashed_source("deferred-full-hash");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");

        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
                .expect("publish pending identity")
        );
        let pending_stages = readiness_stages_for_identity(
            &connection,
            source.id.as_str(),
            "identity-deferred-full-hash",
        );
        assert_eq!(
            pending_stages,
            vec![
                String::from("analysis_features"),
                String::from("embedding_aspects"),
                String::from("indexed_identity"),
                String::from("playback_summary"),
            ]
        );
        let pending_content_generations = {
            let mut statement = connection
                .prepare(
                    "SELECT DISTINCT content_generation
                     FROM source_readiness_targets
                     WHERE source_id = ?1 AND scope_id = 'identity-deferred-full-hash'",
                )
                .expect("prepare pending content generations");
            statement
                .query_map([source.id.as_str()], |row| row.get::<_, String>(0))
                .expect("query pending content generations")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect pending content generations")
        };
        assert_eq!(pending_content_generations.len(), 1);
        assert!(pending_content_generations[0].starts_with("pending-"));
        let pending_membership: String = connection
            .query_row(
                "SELECT content_generation FROM source_readiness_targets
                 WHERE source_id = ?1 AND scope_kind = 'source'",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read pending membership");

        connection
            .execute(
                "UPDATE wav_files SET content_hash = 'full-content-hash'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("commit full content identity");
        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 101)
                .expect("publish full identity")
        );
        let exact_stages = readiness_stages_for_identity(
            &connection,
            source.id.as_str(),
            "identity-deferred-full-hash",
        );
        assert_eq!(
            exact_stages,
            vec![
                String::from("analysis_features"),
                String::from("embedding_aspects"),
                String::from("indexed_identity"),
                String::from("playback_summary"),
            ]
        );
        let exact_membership: String = connection
            .query_row(
                "SELECT content_generation FROM source_readiness_targets
                 WHERE source_id = ?1 AND scope_kind = 'source'",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read exact membership");
        assert_ne!(pending_membership, exact_membership);
    }

    #[test]
    fn unsupported_exact_content_is_terminal_and_excluded_from_similarity_membership() {
        let (_directory, source) = unhashed_source("unsupported-membership");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        connection
            .execute(
                "UPDATE wav_files SET content_hash = 'unsupported-content'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("commit unsupported content identity");
        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
                .expect("publish exact targets")
        );
        let snapshot = reconcile_readiness(&connection, source.id.as_str(), 100)
            .expect("reconcile exact targets");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, 100)
            .expect("persist exact work");
        connection
            .execute(
                "UPDATE analysis_jobs
                 SET status = 'failed', failure_kind = 'unsupported',
                     last_error = 'unsupported codec'
                 WHERE readiness_managed = 1
                   AND readiness_scope_id = 'identity-unsupported-membership'
                   AND readiness_stage = 'analysis_features'",
                [],
            )
            .expect("record terminal unsupported content");

        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), 101)
                .expect("republish unsupported eligibility")
        );
        let embedding_eligibility: String = connection
            .query_row(
                "SELECT eligibility FROM source_readiness_targets
                 WHERE source_id = ?1
                   AND scope_id = 'identity-unsupported-membership'
                   AND stage = 'embedding_aspects'",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read terminal embedding eligibility");
        assert_eq!(embedding_eligibility, "unsupported");
        let source_membership: String = connection
            .query_row(
                "SELECT content_generation FROM source_readiness_targets
                 WHERE source_id = ?1 AND scope_kind = 'source'",
                [source.id.as_str()],
                |row| row.get(0),
            )
            .expect("read supported source membership");
        assert_eq!(
            source_membership,
            blake3::Hasher::new().finalize().to_hex().to_string()
        );
    }

    #[test]
    fn missing_analysis_payload_requeues_its_prerequisite_without_consuming_a_retry() {
        let (_directory, source) = unhashed_source("missing-analysis-payload");
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
                 SET file_identity = 'missing-payload-identity',
                     content_hash = 'missing-payload-content'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("assign readiness identity");
        let now = now_epoch_seconds();
        assert!(
            publish_current_readiness_targets(&mut connection, source.id.as_str(), now)
                .expect("publish current targets")
        );
        let snapshot =
            reconcile_readiness(&connection, source.id.as_str(), now).expect("reconcile targets");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, now)
            .expect("persist readiness work");
        let analysis = snapshot
            .entries
            .iter()
            .find(|entry| entry.target.stage == ReadinessStage::AnalysisFeatures)
            .expect("analysis target")
            .target
            .clone();
        let embedding = snapshot
            .entries
            .iter()
            .find(|entry| entry.target.stage == ReadinessStage::EmbeddingAspects)
            .expect("embedding target")
            .target
            .clone();
        assert_eq!(
            publish_readiness_artifact(
                &mut connection,
                &ReadinessArtifact::for_target(&analysis, now),
            )
            .expect("publish inconsistent analysis marker"),
            ArtifactPublishOutcome::Recorded
        );
        drop(connection);

        let outcome = execute_readiness_target(&source, &embedding, &AtomicBool::new(false))
            .expect("repair inconsistent prerequisite");
        assert_eq!(outcome, ExecutionOutcome::PrerequisiteInvalidated);

        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen readiness database");
        let repaired =
            reconcile_readiness(&connection, source.id.as_str(), now + 1).expect("repair snapshot");
        let repaired_analysis = repaired
            .entries
            .iter()
            .find(|entry| entry.target.stage == ReadinessStage::AnalysisFeatures)
            .expect("repaired analysis target");
        assert_ne!(
            repaired_analysis.classification,
            wavecrate::sample_sources::readiness::ReadinessClassification::Current
        );
        let stats = readiness_work_stats(&connection, now + 1).expect("repaired work stats");
        assert_eq!(stats.cancelled, 0);
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
            coordinator_wait_duration(Some(105), 100, SAFETY_SWEEP_INTERVAL, false),
            Duration::from_secs(5)
        );
        assert_eq!(
            coordinator_wait_duration(Some(100), 100, SAFETY_SWEEP_INTERVAL, false),
            Duration::ZERO
        );
        assert_eq!(
            coordinator_wait_duration(Some(200), 100, SAFETY_SWEEP_INTERVAL, false),
            SAFETY_SWEEP_INTERVAL
        );
        assert_eq!(
            coordinator_wait_duration(None, 100, SAFETY_SWEEP_INTERVAL, false),
            SAFETY_SWEEP_INTERVAL
        );
        assert_eq!(
            coordinator_wait_duration(None, 100, Duration::from_secs(3), false),
            Duration::from_secs(3),
            "priority wakes must preserve the remaining absolute safety-sweep deadline"
        );
        assert_eq!(
            coordinator_wait_duration(Some(100), 100, SAFETY_SWEEP_INTERVAL, true),
            SAFETY_SWEEP_INTERVAL,
            "paused processing must not spin on an already-due retry deadline"
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

    #[test]
    fn deterministic_audio_decode_failures_are_not_retried() {
        for reason in [
            "failed to decode audio file: Invalid wav: no RIFF tag found",
            "Source analysis process failed: Audio decode failed for bad.wav",
            "audio file contains no complete frames",
            "source analysis process failed: unsupported codec",
            "Symphonia probe failed: no suitable format reader found",
        ] {
            assert_eq!(
                readiness_failure_classification(reason),
                ReadinessFailureClassification::Unsupported,
                "{reason}"
            );
        }
        assert_eq!(
            readiness_failure_classification("database is locked"),
            ReadinessFailureClassification::Retryable
        );
        assert_eq!(
            readiness_failure_classification("failed to read audio file: permission denied"),
            ReadinessFailureClassification::Retryable
        );
    }

    #[test]
    fn discovery_self_heals_existing_unsupported_audio_retries() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("open test database");
        connection
            .execute_batch(
                "CREATE TABLE analysis_jobs (
                    id INTEGER PRIMARY KEY,
                    source_id TEXT NOT NULL,
                    readiness_managed INTEGER NOT NULL,
                    readiness_scope_kind TEXT,
                    readiness_scope_id TEXT,
                    readiness_stage TEXT,
                    content_generation TEXT,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL,
                    failure_kind TEXT,
                    retry_at INTEGER,
                    last_error TEXT
                );
                INSERT INTO analysis_jobs VALUES
                    (1, 'source', 1, 'file', 'bad-audio', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', 500,
                        'failed to decode audio file: Invalid wav'),
                    (2, 'source', 1, 'file', 'transient', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', 500, 'database is locked'),
                    (3, 'source', 1, 'file', 'pending', 'analysis_features', 'hash',
                        'pending', 0, NULL, NULL, NULL),
                    (4, 'source', 0, 'file', 'legacy', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', 500, 'unsupported codec'),
                    (5, 'source', 1, 'file', 'bad-audio', 'embedding_aspects', 'hash',
                        'failed', 3, 'retryable', 500,
                        'embedding feature prerequisite is not durable yet'),
                    (6, 'source', 1, 'file', 'missing-payload', 'embedding_aspects', 'hash',
                        'failed', 8, 'permanent', NULL,
                        'embedding feature prerequisite is not durable yet'),
                    (7, 'source', 1, 'file', 'legacy-permanent', 'analysis_features', 'hash',
                        'failed', 8, 'permanent', NULL,
                        'Audio decode failed for empty.wav: no suitable format reader found');",
            )
            .expect("seed readiness failures");

        assert_eq!(
            reclassify_known_unsupported_audio_failures(&mut connection)
                .expect("reclassify unsupported failures"),
            4
        );
        let first = connection
            .query_row(
                "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("read reclassified failure");
        assert_eq!(first, (String::from("unsupported"), None));
        let second = connection
            .query_row(
                "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 2",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("read retryable failure");
        assert_eq!(second, (String::from("retryable"), Some(500)));
        let dependent = connection
            .query_row(
                "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 5",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("read unsupported dependent failure");
        assert_eq!(dependent, (String::from("unsupported"), None));
        let legacy_permanent = connection
            .query_row(
                "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 7",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("read legacy permanent failure");
        assert_eq!(legacy_permanent, (String::from("unsupported"), None));
        let repaired = connection
            .query_row(
                "SELECT failure_kind, attempts, retry_at FROM analysis_jobs WHERE id = 6",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .expect("read repaired prerequisite failure");
        assert_eq!(repaired, (String::from("retryable"), 0, None));
        let target = ReadinessTarget::file(
            "source",
            "bad-audio",
            "bad.wav",
            ReadinessStage::EmbeddingAspects,
            "embedding-v1",
            1,
            "hash",
        );
        assert!(
            readiness_stage_is_unsupported(&connection, &target, "analysis_features")
                .expect("read unsupported prerequisite")
        );
        assert_eq!(
            reclassify_known_unsupported_audio_failures(&mut connection)
                .expect("reclassification is idempotent"),
            0
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

    fn readiness_stages_for_identity(
        connection: &rusqlite::Connection,
        source_id: &str,
        identity: &str,
    ) -> Vec<String> {
        let mut statement = connection
            .prepare(
                "SELECT stage FROM source_readiness_targets
                 WHERE source_id = ?1 AND scope_id = ?2
                 ORDER BY stage",
            )
            .expect("prepare identity readiness stages");
        statement
            .query_map(params![source_id, identity], |row| row.get(0))
            .expect("query identity readiness stages")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect identity readiness stages")
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
