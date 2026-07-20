#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{OptionalExtension, params};
use serde_json::Value;
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole, SourceMetadataStorage,
    db::{META_LAST_MANIFEST_AUDIT_AT, META_WAV_PATHS_REVISION},
    readiness::{
        ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessClassification,
        ReadinessDeltaPublicationOutcome, ReadinessEligibility, ReadinessFailureClassification,
        ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome, ReadinessMembership,
        ReadinessRetryPolicy, ReadinessScopeKind, ReadinessSnapshot, ReadinessStage,
        ReadinessStore, ReadinessTarget, ReadinessTargetDeltaPublication,
        ReadinessTargetPublication, ReadinessWorkMutationOutcome, SourceAvailability,
    },
    scanner::{
        CommittedSourceDelta, ScanError, audit_source_and_record_with_progress,
        complete_pending_deep_hash_for_path, sync_paths_with_progress,
    },
};

use super::worker::{SourceProcessingFailure, source_database_failure};
use super::{
    SourceProcessingActivity, SourceProcessingEvent, SourceProcessingEventSink,
    SourceProcessingLifecycle, SourceProcessingProgressEvent,
    scheduler::{
        BudgetTracker, FairScheduler, PriorityContext, ProcessingBudgets, ProcessingLane,
        WorkCandidate,
    },
};
use crate::native_app::sample_library::similarity_artifacts::{
    SimilarityPublicationFence, finalize_similarity_artifacts_if_ready,
    native_similarity_artifact_version,
};
use crate::native_app::waveform::invalidate_persisted_waveform_cache_ref;

const SAFETY_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const PROGRESS_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL: Duration = Duration::from_millis(250);
const DISCOVERY_PROGRESS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const DISCOVERY_PROGRESS_LOG_INTERVAL: Duration = Duration::from_secs(2);
const SIMILARITY_SCORE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const MANIFEST_AUDIT_INTERVAL_SECONDS: i64 = 24 * 60 * 60;
const MANIFEST_AUDIT_HASH_BATCH: usize = 8;
const MAX_VISIBLE_PRIORITY_PATHS: usize = 128;
const READINESS_LEASE_SECONDS: i64 = 5 * 60;
const READINESS_MAX_ATTEMPTS: u32 = 8;
const READINESS_MANIFEST_VERSION: &str = "source_manifest_v1";
const READINESS_MEMBERSHIP_VERSION: &str = "membership-xor-v1";
const SOURCE_RETIREMENT_RETRY_SECONDS: i64 = 5;
const SOURCE_DISCOVERY_RETRY_SECONDS: i64 = 5;
const PREREQUISITE_INVALIDATION_RETRY_SECONDS: i64 = 5;
const ACTIVE_RECORDING_QUIET_SECONDS: i64 = 5;
const ORPHAN_CACHE_MIN_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const ORPHAN_CACHE_MAX_SCANNED: usize = 4_096;
const ORPHAN_CACHE_MAX_REMOVED: usize = 32;
const RETAINED_SOURCE_MAX_SCANNED: usize = 1_024;
static ORPHAN_CACHE_SCAN_CURSOR: AtomicUsize = AtomicUsize::new(0);

/// Owned runtime coordinator. All work is joined during shutdown and observes one cancel token.
pub(in crate::native_app) struct SourceProcessingSupervisor {
    shared: Arc<Shared>,
    coordinator: Option<JoinHandle<()>>,
    retirement_worker: Option<JoinHandle<()>>,
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
    cancel: Arc<AtomicBool>,
    retry_at: i64,
    attempts: u32,
    terminal_offline: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PendingReadinessDelta {
    scope_ids: BTreeSet<String>,
}

impl PendingReadinessDelta {
    fn merge(&mut self, delta: &CommittedSourceDelta) {
        self.scope_ids.extend(
            delta
                .created
                .iter()
                .chain(&delta.changed)
                .chain(&delta.deleted)
                .map(|entry| entry.identity.clone()),
        );
        self.scope_ids
            .extend(delta.moved.iter().map(|entry| entry.identity.clone()));
    }

    fn is_empty(&self) -> bool {
        self.scope_ids.is_empty()
    }
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
    #[cfg(test)]
    pub(in crate::native_app) fn lifecycle_generation(&self, source_id: &str) -> Option<u64> {
        self.shared
            .control()
            .source_lifecycle_generations
            .get(source_id)
            .copied()
    }

    pub(in crate::native_app) fn acquire_scan_for_generation(
        &self,
        source_id: &str,
        expected_lifecycle_generation: u64,
    ) -> Option<SourceProcessingBudgetPermit> {
        {
            let mut control = self.shared.control();
            while !control.shutdown
                && control.source_is_configured(source_id)
                && control.source_lifecycle_generations.get(source_id)
                    == Some(&expected_lifecycle_generation)
                && !control.source_is_active(source_id)
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
                || control.source_lifecycle_generations.get(source_id)
                    != Some(&expected_lifecycle_generation)
            {
                return None;
            }
        }
        // Publish the admission while holding the budget lock so the coordinator cannot start
        // another candidate between observing capacity and observing the external scan. Existing
        // source work is allowed to finish; watcher and UI scans must never cancel the active
        // source merely to acquire the lane sooner.
        let budgets = self.shared.budgets();
        let (admission_id, admission_cancel, lifecycle_generation) = {
            let control = self.shared.control();
            if control.shutdown
                || self.shared.cancel.load(Ordering::Acquire)
                || !control.source_is_active(source_id)
                || control.source_lifecycle_generations.get(source_id)
                    != Some(&expected_lifecycle_generation)
            {
                return None;
            }
            let admission_cancel = Arc::clone(&control.source_work_cancels[source_id]);
            let lifecycle_generation = expected_lifecycle_generation;
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
                || admission_cancel.load(Ordering::Acquire);
            drop(control);
            if unavailable {
                self.shared.finish_external_scan_admission(admission_id);
                return None;
            }
        }
    }

    /// Wait for source replacement to finish, then resolve the descriptor
    /// against the authoritative configured set.
    ///
    /// This is intentionally exposed on the background-only budget handle:
    /// source replacement briefly fences retirement admission and publication,
    /// so callers on the UI thread must use the supervisor's non-blocking
    /// registration method instead.
    pub(in crate::native_app) fn register_source_for_scan_waiting(
        &self,
        source: SampleSource,
    ) -> Result<u64, String> {
        let _replacement = self
            .shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        resolve_registered_source_for_scan_locked(self.shared.as_ref(), &source)
    }

    #[cfg(test)]
    pub(in crate::native_app) fn acquire_scan(
        &self,
        source_id: &str,
    ) -> Option<SourceProcessingBudgetPermit> {
        let lifecycle_generation = self.lifecycle_generation(source_id)?;
        self.acquire_scan_for_generation(source_id, lifecycle_generation)
    }
}

impl SourceProcessingBudgetPermit {
    pub(in crate::native_app) fn cancel_token(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    pub(in crate::native_app) fn lifecycle_generation(&self) -> u64 {
        self.lifecycle_generation
    }

    fn should_cancel_now(&self) -> bool {
        if self.shared.cancel.load(Ordering::Acquire) {
            return true;
        }
        let control = self.shared.control();
        control.shutdown
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

fn install_worker_app_root(app_root: PathBuf) -> wavecrate::app_dirs::AppRootGuard {
    wavecrate::app_dirs::AppRootGuard::set(app_root)
        .expect("source-processing worker should inherit the resolved persistence root")
}

impl SourceProcessingSupervisor {
    #[cfg(test)]
    pub(in crate::native_app) fn start(sources: Vec<SampleSource>) -> Self {
        Self::start_with_playback_state(sources, false)
    }

    pub(in crate::native_app) fn start_with_event_sink(
        sources: Vec<SampleSource>,
        event_sink: impl SourceProcessingEventSink + 'static,
    ) -> Self {
        Self::start_with_options(sources, false, Some(Arc::new(event_sink)), false)
    }

    #[cfg(test)]
    fn start_with_playback_state(sources: Vec<SampleSource>, playback_active: bool) -> Self {
        Self::start_with_playback_state_and_event_sink(sources, playback_active, None)
    }

    #[cfg(test)]
    fn start_with_playback_state_and_event_sink(
        sources: Vec<SampleSource>,
        playback_active: bool,
        event_sink: Option<Arc<dyn SourceProcessingEventSink>>,
    ) -> Self {
        Self::start_with_options(sources, playback_active, event_sink, false)
    }

    fn start_with_options(
        sources: Vec<SampleSource>,
        playback_active: bool,
        event_sink: Option<Arc<dyn SourceProcessingEventSink>>,
        synthetic_test_execution: bool,
    ) -> Self {
        let app_root = wavecrate::app_dirs::app_root_dir()
            .expect("source-processing supervisor should resolve its persistence root");
        let shared = Arc::new(Shared::new(sources, event_sink));
        shared.control().playback_active = playback_active;
        shared
            .synthetic_test_execution
            .store(synthetic_test_execution, Ordering::Release);
        let thread_shared = Arc::clone(&shared);
        let coordinator_app_root = app_root.clone();
        let coordinator = thread::Builder::new()
            .name(String::from("wavecrate-source-supervisor"))
            .spawn(move || {
                let _app_root_guard = install_worker_app_root(coordinator_app_root);
                run_coordinator(thread_shared);
            })
            .expect("spawn source processing supervisor");
        let retirement_shared = Arc::clone(&shared);
        let retirement_app_root = app_root;
        let retirement_worker = thread::Builder::new()
            .name(String::from("wavecrate-source-retirement"))
            .spawn(move || {
                let _app_root_guard = install_worker_app_root(retirement_app_root);
                run_retirement_worker(retirement_shared);
            })
            .expect("spawn source retirement worker");
        Self {
            shared,
            coordinator: Some(coordinator),
            retirement_worker: Some(retirement_worker),
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn dormant() -> Self {
        Self {
            shared: Arc::new(Shared::new(Vec::new(), None)),
            coordinator: None,
            retirement_worker: None,
        }
    }

    #[cfg(any(test, feature = "legacy-controller"))]
    pub(in crate::native_app) fn is_running(&self) -> bool {
        self.coordinator.is_some() && self.retirement_worker.is_some()
    }

    #[cfg(test)]
    fn start_synthetic_profile(sources: Vec<SampleSource>, playback_active: bool) -> Self {
        Self::start_with_options(sources, playback_active, None, true)
    }

    #[cfg(test)]
    fn start_without_forced_manifest_audit(sources: Vec<SampleSource>) -> Self {
        let shared = Arc::new(Shared::new(sources, None));
        shared.control().force_manifest_audit_sources.clear();
        shared.control().force_reanalysis_sources.clear();
        let thread_shared = Arc::clone(&shared);
        let coordinator = thread::Builder::new()
            .name(String::from("wavecrate-source-supervisor"))
            .spawn(move || run_coordinator(thread_shared))
            .expect("spawn source processing supervisor");
        let retirement_shared = Arc::clone(&shared);
        let retirement_worker = thread::Builder::new()
            .name(String::from("wavecrate-source-retirement"))
            .spawn(move || run_retirement_worker(retirement_shared))
            .expect("spawn source retirement worker");
        Self {
            shared,
            coordinator: Some(coordinator),
            retirement_worker: Some(retirement_worker),
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
                    cancel: Arc::new(AtomicBool::new(false)),
                    retry_at: 0,
                    attempts: 0,
                    terminal_offline: false,
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
        for retirement in control.pending_retirements.values() {
            if sources
                .values()
                .any(|active| source_storage_identity_matches(active, &retirement.source))
            {
                retirement.cancel.store(true, Ordering::Release);
            }
        }
        control.sources = sources;
        control.source_work_cancels = source_work_cancels;
        control.source_lifecycle_generations = source_lifecycle_generations;
        control.quarantined_sources.clear();
        let retained_source_ids = control.sources.keys().cloned().collect::<BTreeSet<_>>();
        control
            .force_manifest_audit_sources
            .retain(|source_id| retained_source_ids.contains(source_id));
        control
            .force_reanalysis_sources
            .retain(|source_id| retained_source_ids.contains(source_id));
        control.force_manifest_audit_sources.extend(
            changed_source_ids
                .iter()
                .filter(|source_id| retained_source_ids.contains(*source_id))
                .cloned(),
        );
        control
            .dirty_sources
            .retain(|source_id| retained_source_ids.contains(source_id));
        control.safety_probe_sources.retain(|source_id| {
            retained_source_ids.contains(source_id) && !changed_source_ids.contains(source_id)
        });
        control.pending_readiness_deltas.retain(|source_id, _| {
            retained_source_ids.contains(source_id) && !changed_source_ids.contains(source_id)
        });
        control
            .awaiting_foreground_refresh_sources
            .retain(|source_id| {
                retained_source_ids.contains(source_id) && !changed_source_ids.contains(source_id)
            });
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
        self.shared.retirement_wake.notify_all();
        Ok(())
    }

    /// Admit a newly configured source before its first external scan starts.
    ///
    /// This deliberately only grows the configured set. Full replacement also
    /// retires removed lifecycle epochs and is owned by the configuration path.
    pub(in crate::native_app) fn register_source_for_scan(
        &self,
        source: SampleSource,
    ) -> Result<u64, String> {
        let _replacement = match self.shared.source_replacement.try_lock() {
            Ok(replacement) => replacement,
            Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err("Configured sources are currently being replaced".to_string());
            }
        };
        register_source_for_scan_locked(self.shared.as_ref(), source)
    }

    pub(in crate::native_app) fn budget_handle(&self) -> SourceProcessingBudgetHandle {
        SourceProcessingBudgetHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(in crate::native_app) fn lifecycle_generations(&self) -> BTreeMap<String, u64> {
        self.shared.control().source_lifecycle_generations.clone()
    }

    /// Re-arm the authoritative source audits after the watcher stream is live.
    ///
    /// The initial audit and this watcher-ready request coalesce while an audit
    /// is in flight. If it already completed, this request runs one final audit
    /// that closes the gap between its snapshot and native event delivery.
    pub(in crate::native_app) fn request_manifest_audits(&self, reason: &'static str) {
        let mut control = self.shared.control();
        let active_source_ids = control
            .sources
            .keys()
            .filter(|source_id| control.source_is_active(source_id))
            .cloned()
            .collect::<Vec<_>>();
        if active_source_ids.is_empty() {
            return;
        }
        control
            .force_manifest_audit_sources
            .extend(active_source_ids.iter().cloned());
        control.dirty_sources.extend(active_source_ids);
        control.notify(reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn wake_source(&self, source_id: &str, reason: &'static str) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        let bounded_delta_pending = control.pending_readiness_deltas.contains_key(source_id);
        if !bounded_delta_pending {
            control.cancel_source_work(source_id);
        }
        control.mark_source_dirty(source_id, reason);
        drop(control);
        if !bounded_delta_pending {
            self.shared
                .cancel_external_scans(|registration| registration.source_id == source_id);
            self.shared.budget_wake.notify_all();
        }
        self.shared.wake.notify_one();
    }

    /// Force complete source reconciliation when a bounded delta cannot describe all changes.
    pub(in crate::native_app) fn wake_source_for_full_reconciliation(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        self.shared
            .control()
            .pending_readiness_deltas
            .remove(source_id);
        self.wake_source(source_id, reason);
    }

    /// Reconcile a source without invalidating work that already owns its
    /// current lifecycle generation.
    ///
    /// UI projection refreshes and completed foreground scans can arrive well
    /// after the source database commit that scheduled them. Treating those
    /// delayed notifications as a new mutation would kill a valid long-running
    /// finalizer and immediately start the same work again.
    pub(in crate::native_app) fn request_source_processing(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    /// Publish the affected identity set from one authoritative committed manifest delta.
    pub(in crate::native_app) fn request_source_delta(
        &self,
        source_id: &str,
        delta: &CommittedSourceDelta,
        reason: &'static str,
    ) {
        if delta.is_empty() {
            return;
        }
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control
            .pending_readiness_deltas
            .entry(source_id.to_string())
            .or_default()
            .merge(delta);
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    /// Requeue exact current feature, embedding, and similarity targets after
    /// an explicit user request.
    pub(in crate::native_app) fn request_source_reanalysis(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.cancel_source_work(source_id);
        control
            .force_reanalysis_sources
            .insert(source_id.to_string());
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared
            .cancel_external_scans(|registration| registration.source_id == source_id);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn finish_foreground_source_refresh(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control
            .awaiting_foreground_refresh_sources
            .remove(source_id);
        control.mark_source_dirty(source_id, reason);
        drop(control);
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

    #[cfg(test)]
    pub(in crate::native_app) fn selected_source_priority_for_tests(&self) -> Option<String> {
        self.shared.control().priority.selected_source.clone()
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
            control.notify("playback_activity_changed");
            tracing::info!(
                target: "wavecrate::source_processing",
                event = "source_processing.playback_activity_changed",
                active,
                "Playback activity changed without pausing source processing"
            );
            drop(control);
            self.shared.wake.notify_all();
        }
    }

    pub(in crate::native_app) fn set_foreground_activity(&self, active: bool) {
        let mut control = self.shared.control();
        if control.foreground_active == active {
            return;
        }
        control.foreground_active = active;
        control.notify("foreground_activity_changed");
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.foreground_activity_changed",
            active,
            "Foreground loading activity changed without pausing source processing"
        );
        drop(control);
        self.shared.wake.notify_all();
    }

    pub(in crate::native_app) fn shutdown(&mut self) -> Value {
        let started_at = Instant::now();
        self.shared.cancel.store(true, Ordering::Release);
        self.shared.cancel_external_scans(|_| true);
        {
            let mut control = self.shared.control();
            control.cancel_all_source_work();
            for retirement in control.pending_retirements.values() {
                retirement.cancel.store(true, Ordering::Release);
            }
            control.shutdown = true;
            control.notify("shutdown");
        }
        self.shared.wake.notify_all();
        self.shared.budget_wake.notify_all();
        self.shared.retirement_wake.notify_all();
        let coordinator_joined = self
            .coordinator
            .take()
            .is_none_or(|coordinator| coordinator.join().is_ok());
        let retirement_joined = self
            .retirement_worker
            .take()
            .is_none_or(|worker| worker.join().is_ok());
        let joined = coordinator_joined && retirement_joined;
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
            "queue_depth_by_source": telemetry.queue_depth_by_source,
            "readiness_queue_depth_by_source": telemetry.readiness_queue_depth_by_source,
            "retries_due_by_source": telemetry.retries_due_by_source,
            "retry_at_by_source": telemetry.retry_at_by_source,
            "source_discoveries": telemetry.source_discoveries,
            "cheap_noop_sweeps": telemetry.cheap_noop_sweeps,
            "delta_reconciliations": telemetry.delta_reconciliations,
            "full_audits": telemetry.full_audits,
            "settled_wake_generation": telemetry.settled_wake_generation,
        })
    }
}

impl Drop for SourceProcessingSupervisor {
    fn drop(&mut self) {
        if self.coordinator.is_some() || self.retirement_worker.is_some() {
            let _ = self.shutdown();
        }
    }
}

struct Shared {
    source_replacement: Mutex<()>,
    state: Mutex<ControlState>,
    wake: Condvar,
    retirement_wake: Condvar,
    cancel: AtomicBool,
    telemetry: Mutex<SupervisorTelemetry>,
    budgets: Mutex<BudgetTracker>,
    budget_wake: Condvar,
    external_scans: Mutex<ExternalScanState>,
    external_scan_wake: Condvar,
    next_external_scan_id: AtomicU64,
    in_flight_work: Mutex<BTreeMap<(String, u64), usize>>,
    synthetic_test_execution: AtomicBool,
    event_sink: Option<Arc<dyn SourceProcessingEventSink>>,
    #[cfg(test)]
    retirement_cleanup_blocked: AtomicBool,
    #[cfg(test)]
    retirement_cleanup_started: AtomicBool,
}

impl Shared {
    fn new(
        sources: Vec<SampleSource>,
        event_sink: Option<Arc<dyn SourceProcessingEventSink>>,
    ) -> Self {
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
        let force_manifest_audit_sources = sources.keys().cloned().collect();
        Self {
            source_replacement: Mutex::new(()),
            state: Mutex::new(ControlState {
                sources,
                source_work_cancels,
                source_lifecycle_generations,
                next_lifecycle_generation,
                dirty_sources,
                safety_probe_sources: BTreeSet::new(),
                pending_readiness_deltas: BTreeMap::new(),
                awaiting_foreground_refresh_sources: BTreeSet::new(),
                force_manifest_audit_sources,
                force_reanalysis_sources: BTreeSet::new(),
                quarantined_sources: BTreeSet::new(),
                pending_retirements,
                next_retirement_id,
                wake_generation: 1,
                wake_reason: "startup",
                playback_active: false,
                foreground_active: false,
                shutdown: false,
                priority: PriorityContext::default(),
            }),
            wake: Condvar::new(),
            retirement_wake: Condvar::new(),
            cancel: AtomicBool::new(false),
            telemetry: Mutex::new(SupervisorTelemetry::default()),
            budgets: Mutex::new(BudgetTracker::new(ProcessingBudgets::default())),
            budget_wake: Condvar::new(),
            external_scans: Mutex::new(ExternalScanState::default()),
            external_scan_wake: Condvar::new(),
            next_external_scan_id: AtomicU64::new(1),
            in_flight_work: Mutex::new(BTreeMap::new()),
            synthetic_test_execution: AtomicBool::new(false),
            event_sink,
            #[cfg(test)]
            retirement_cleanup_blocked: AtomicBool::new(false),
            #[cfg(test)]
            retirement_cleanup_started: AtomicBool::new(false),
        }
    }

    fn control(&self) -> MutexGuard<'_, ControlState> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn publish_event(&self, event: SourceProcessingEvent) -> bool {
        let lifecycle_guard = if let Some(lifecycle) = event.lifecycle() {
            let control = self.control();
            if !control.source_is_active(&lifecycle.source_id)
                || control
                    .source_lifecycle_generations
                    .get(&lifecycle.source_id)
                    != Some(&lifecycle.generation)
            {
                return false;
            }
            Some(control)
        } else {
            None
        };
        let published = self
            .event_sink
            .as_ref()
            .is_some_and(|sink| sink.try_publish(event));
        drop(lifecycle_guard);
        published
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
    safety_probe_sources: BTreeSet<String>,
    pending_readiness_deltas: BTreeMap<String, PendingReadinessDelta>,
    awaiting_foreground_refresh_sources: BTreeSet<String>,
    force_manifest_audit_sources: BTreeSet<String>,
    force_reanalysis_sources: BTreeSet<String>,
    quarantined_sources: BTreeSet<String>,
    pending_retirements: BTreeMap<u64, PendingSourceRetirement>,
    next_retirement_id: u64,
    wake_generation: u64,
    wake_reason: &'static str,
    playback_active: bool,
    foreground_active: bool,
    shutdown: bool,
    priority: PriorityContext,
}

impl ControlState {
    fn source_is_configured(&self, source_id: &str) -> bool {
        self.sources.contains_key(source_id) && !self.quarantined_sources.contains(source_id)
    }

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
            self.safety_probe_sources.remove(source_id);
            self.dirty_sources.insert(source_id.to_string());
            self.notify(reason);
        }
    }

    fn mark_all_sources_dirty(&mut self, reason: &'static str) {
        self.safety_probe_sources.clear();
        self.dirty_sources.extend(
            self.sources
                .keys()
                .filter(|source_id| !self.quarantined_sources.contains(*source_id))
                .cloned(),
        );
        self.notify(reason);
    }

    fn mark_all_sources_for_safety_probe(&mut self) {
        let source_ids = self
            .sources
            .keys()
            .filter(|source_id| !self.quarantined_sources.contains(*source_id))
            .cloned()
            .collect::<Vec<_>>();
        self.safety_probe_sources.extend(source_ids.iter().cloned());
        self.dirty_sources.extend(source_ids);
        self.notify("periodic_safety_sweep");
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
    queue_depth_by_source: BTreeMap<String, usize>,
    readiness_queue_depth_by_source: BTreeMap<String, usize>,
    retries_due_by_source: BTreeMap<String, usize>,
    retry_at_by_source: BTreeMap<String, i64>,
    source_discoveries: u64,
    cheap_noop_sweeps: u64,
    delta_reconciliations: u64,
    full_audits: u64,
    settled_wake_generation: u64,
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
    prerequisites_blocked: usize,
    prerequisite_retry_at: Option<i64>,
    retries_due: usize,
    earliest_retry_at: Option<i64>,
    progress_completed: usize,
    progress_total: usize,
    cheap_noop_sweep: bool,
    delta_reconciled: bool,
}

enum Cancellable<T> {
    Completed(T),
    Cancelled,
}

struct DiscoveryProgressPublisher<'a> {
    shared: &'a Shared,
    source_id: &'a str,
    lifecycle_generation: u64,
    started_at: Instant,
    last_phase: Option<&'static str>,
    last_event_publish_at: Option<Instant>,
    last_log_publish_at: Option<Instant>,
    event_published: bool,
}

impl DiscoveryProgressPublisher<'_> {
    fn advance(&mut self, phase: &'static str, work_units: usize) {
        let phase_changed = self.last_phase != Some(phase);
        let event_due = self.started_at.elapsed() >= DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL
            && (phase_changed
                || self.last_event_publish_at.is_none_or(|published_at| {
                    published_at.elapsed() >= DISCOVERY_PROGRESS_REFRESH_INTERVAL
                }));
        if event_due {
            self.event_published |= self.shared.publish_event(SourceProcessingEvent::Progress(
                SourceProcessingProgressEvent {
                    lifecycle: SourceProcessingLifecycle::new(
                        self.source_id,
                        self.lifecycle_generation,
                    ),
                    source_row_active: true,
                    completed: 0,
                    total: 0,
                    activity: SourceProcessingActivity::Discovering {
                        phase: phase.to_string(),
                        completed_steps: work_units,
                    },
                },
            ));
            self.last_event_publish_at = Some(Instant::now());
        }
        let log_due = phase_changed
            || self.last_log_publish_at.is_none_or(|published_at| {
                published_at.elapsed() >= DISCOVERY_PROGRESS_LOG_INTERVAL
            });
        if log_due {
            tracing::info!(
                target: "wavecrate::source_processing",
                event = "source_processing.discovery_progress",
                source_id = self.source_id,
                lifecycle_generation = self.lifecycle_generation,
                phase,
                work_units,
                "Source discovery reconciliation advanced"
            );
            self.last_log_publish_at = Some(Instant::now());
        }
        self.last_phase = Some(phase);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutionOutcome {
    Completed,
    CompletedAwaitingForegroundRefresh,
    Retried { retry_at: i64 },
    Failed,
    FailedAwaitingForegroundRefresh,
    PrerequisiteInvalidated { retry_at: i64, reason: &'static str },
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
    let mut candidates = Vec::<RuntimeCandidate>::new();
    let mut source_stats = BTreeMap::<String, SourceDiscoveryStats>::new();
    let mut active_progress_source = None::<String>;
    let mut last_progress_publish_at = None::<Instant>;
    let mut pending_similarity_refresh_lifecycles = BTreeSet::<SourceProcessingLifecycle>::new();
    let mut pending_discovery_sources = BTreeSet::<String>::new();
    let mut pending_safety_probe_sources = BTreeSet::<String>::new();
    let mut last_similarity_refresh_publish_at = None::<Instant>;
    let mut progress_visible = false;
    #[cfg(test)]
    let mut synthetic_connections = BTreeMap::<String, rusqlite::Connection>::new();
    loop {
        let (
            sources,
            dirty_sources,
            safety_probe_sources,
            awaiting_foreground_refresh_sources,
            force_manifest_audit_sources,
            force_reanalysis_sources,
            pending_readiness_deltas,
            source_work_cancels,
            source_lifecycle_generations,
            priority,
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
                );
                if progress_visible
                    && !wait_duration.is_zero()
                    && !scheduler.active_source().is_some_and(|source_id| {
                        source_stats.get(source_id).is_some_and(|stats| {
                            stats.prerequisites_blocked > 0 || stats.earliest_retry_at.is_some()
                        })
                    })
                {
                    // Keep feedback stable across immediate coordinator handoffs. Only clear it
                    // when the coordinator is genuinely about to sleep with no newly published
                    // work or prerequisite retry waiting to be handled.
                    publish_source_processing_finished(&shared);
                    progress_visible = false;
                    active_progress_source = None;
                    last_progress_publish_at = None;
                }
                let (next, _) = shared
                    .wake
                    .wait_timeout(control, wait_duration)
                    .unwrap_or_else(|poison| poison.into_inner());
                control = next;
                if control.wake_generation == observed_generation {
                    let now = now_epoch_seconds();
                    if Instant::now() >= next_safety_sweep_at {
                        control.mark_all_sources_for_safety_probe();
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
            let awaiting_foreground_refresh_sources =
                control.awaiting_foreground_refresh_sources.clone();
            let dirty_sources = std::mem::take(&mut control.dirty_sources)
                .into_iter()
                .filter(|source_id| !awaiting_foreground_refresh_sources.contains(source_id))
                .collect::<BTreeSet<_>>();
            let safety_probe_sources = std::mem::take(&mut control.safety_probe_sources)
                .into_iter()
                .filter(|source_id| dirty_sources.contains(source_id))
                .collect::<BTreeSet<_>>();
            let force_manifest_audit_sources = control.force_manifest_audit_sources.clone();
            let force_reanalysis_sources = control.force_reanalysis_sources.clone();
            let pending_readiness_deltas = control.pending_readiness_deltas.clone();
            (
                control
                    .sources
                    .iter()
                    .filter(|(source_id, _)| control.source_is_active(source_id))
                    .map(|(_, source)| source.clone())
                    .collect::<Vec<_>>(),
                dirty_sources,
                safety_probe_sources,
                awaiting_foreground_refresh_sources,
                force_manifest_audit_sources,
                force_reanalysis_sources,
                pending_readiness_deltas,
                control.source_work_cancels.clone(),
                control.source_lifecycle_generations.clone(),
                control.priority.clone(),
                control.wake_generation,
                control.wake_reason,
            )
        };
        observed_generation = generation;
        let configured_source_ids = sources
            .iter()
            .map(|source| source.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        pending_discovery_sources.extend(dirty_sources.iter().cloned());
        for source_id in &dirty_sources {
            if safety_probe_sources.contains(source_id) {
                pending_safety_probe_sources.insert(source_id.clone());
            } else {
                pending_safety_probe_sources.remove(source_id);
            }
        }
        pending_discovery_sources
            .retain(|source_id| !awaiting_foreground_refresh_sources.contains(source_id));
        pending_discovery_sources.retain(|source_id| configured_source_ids.contains(source_id));
        pending_safety_probe_sources
            .retain(|source_id| pending_discovery_sources.contains(source_id));
        let discovery_source_id = select_source_for_discovery(
            &sources,
            &pending_discovery_sources,
            scheduler.active_source(),
            &priority,
        );
        let discovery_is_safety_probe = discovery_source_id
            .as_ref()
            .is_some_and(|source_id| pending_safety_probe_sources.contains(source_id));
        if let Some(source_id) = discovery_source_id.as_ref() {
            pending_discovery_sources.remove(source_id);
            pending_safety_probe_sources.remove(source_id);
        }
        let sources_to_discover = sources
            .iter()
            .filter(|source| discovery_source_id.as_deref() == Some(source.id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        candidates.retain(|candidate| {
            let source_id = candidate.source.id.as_str();
            if !configured_source_ids.contains(source_id) {
                return false;
            }
            if !dirty_sources.contains(source_id) {
                return true;
            }
            pending_readiness_deltas
                .get(source_id)
                .is_some_and(|delta| {
                    candidate.schedule.scope_id != source_id
                        && !delta.scope_ids.contains(&candidate.schedule.scope_id)
                })
        });
        source_stats.retain(|source_id, _| configured_source_ids.contains(source_id));
        let sweep_started = Instant::now();
        for source in &sources_to_discover {
            if !discovery_is_safety_probe
                && !pending_readiness_deltas.contains_key(source.id.as_str())
            {
                source_stats.remove(source.id.as_str());
            }
        }
        let (
            mut discovered,
            mut discovered_source_stats,
            deferred_discoveries,
            consumed_readiness_delta_sources,
            discovery_progress_published,
        ) = discover_candidates(
            &shared,
            &sources_to_discover,
            &force_manifest_audit_sources,
            &force_reanalysis_sources,
            &pending_readiness_deltas,
            discovery_is_safety_probe,
            &source_work_cancels,
        );
        if !consumed_readiness_delta_sources.is_empty() {
            let mut control = shared.control();
            for source_id in consumed_readiness_delta_sources {
                if control.pending_readiness_deltas.get(&source_id)
                    == pending_readiness_deltas.get(&source_id)
                {
                    control.pending_readiness_deltas.remove(&source_id);
                }
            }
        }
        let discovery_deferred_for_capacity = !deferred_discoveries.is_empty();
        if discovery_progress_published {
            progress_visible = true;
            active_progress_source = sources_to_discover
                .first()
                .map(|source| source.id.as_str().to_string());
            last_progress_publish_at = Some(Instant::now());
        }
        pending_discovery_sources.extend(deferred_discoveries);
        if discovery_is_safety_probe {
            pending_safety_probe_sources.extend(
                pending_discovery_sources
                    .iter()
                    .filter(|source_id| {
                        sources_to_discover
                            .iter()
                            .any(|source| source.id.as_str() == source_id.as_str())
                    })
                    .cloned(),
            );
        }
        for (source_id, delta_stats) in &mut discovered_source_stats {
            if !pending_readiness_deltas.contains_key(source_id) {
                continue;
            }
            let Some(previous) = source_stats.get(source_id).copied() else {
                continue;
            };
            let retained_readiness = candidates
                .iter()
                .filter(|candidate| {
                    candidate.source.id.as_str() == source_id
                        && matches!(candidate.task, RuntimeTask::Readiness(_))
                })
                .count();
            delta_stats.readiness_queue_depth = delta_stats
                .readiness_queue_depth
                .saturating_add(retained_readiness);
            delta_stats.progress_total = previous.progress_total.max(delta_stats.progress_total);
            delta_stats.progress_completed = previous
                .progress_completed
                .saturating_sub(delta_stats.readiness_queue_depth);
            delta_stats.earliest_retry_at =
                earliest_deadline(previous.earliest_retry_at, delta_stats.earliest_retry_at);
            delta_stats.retries_due = previous.retries_due.saturating_add(delta_stats.retries_due);
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
            telemetry.queue_depth_by_source = queue_depths_by_source(&candidates);
            telemetry.readiness_queue_depth_by_source = source_stats
                .iter()
                .map(|(source_id, stats)| (source_id.clone(), stats.readiness_queue_depth))
                .collect();
            telemetry.retries_due_by_source = source_stats
                .iter()
                .map(|(source_id, stats)| (source_id.clone(), stats.retries_due))
                .collect();
            telemetry.retry_at_by_source = source_stats
                .iter()
                .filter_map(|(source_id, stats)| {
                    stats
                        .earliest_retry_at
                        .map(|retry_at| (source_id.clone(), retry_at))
                })
                .collect();
        }
        release_converged_source_owner(
            &mut scheduler,
            &configured_source_ids,
            &source_stats,
            &candidates,
        );
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
            let lifecycle_generation = in_flight_work.lifecycle_generation;
            let progress_publish_due = last_progress_publish_at
                .is_none_or(|published_at| published_at.elapsed() >= PROGRESS_REFRESH_INTERVAL);
            if (active_progress_source.as_deref() != Some(candidate.source.id.as_str())
                || !matches!(&candidate.task, RuntimeTask::Readiness(_))
                || progress_publish_due)
                && let Some(progress) = source_stats.get(candidate.source.id.as_str()).copied()
            {
                publish_source_processing_progress(
                    &shared,
                    &candidate,
                    lifecycle_generation,
                    progress,
                );
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
                        &mut synthetic_connections,
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
                Ok(ExecutionOutcome::Completed
                    | ExecutionOutcome::CompletedAwaitingForegroundRefresh)
            ) && matches!(&candidate.task, RuntimeTask::ManifestAudit)
            {
                clear_satisfied_manifest_audit_request(
                    shared.as_ref(),
                    candidate.source.id.as_str(),
                );
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
                        pending_similarity_refresh_lifecycles.insert(
                            SourceProcessingLifecycle::new(
                                target.source_id.clone(),
                                lifecycle_generation,
                            ),
                        );
                    }
                    if outcome.was_claimed() {
                        telemetry.claimed = telemetry.claimed.saturating_add(1);
                    }
                    match outcome {
                        ExecutionOutcome::Completed
                        | ExecutionOutcome::CompletedAwaitingForegroundRefresh => {
                            telemetry.completed = telemetry.completed.saturating_add(1);
                            if matches!(&candidate.task, RuntimeTask::Readiness(_))
                                && let Some(progress) = advance_source_progress(
                                    &mut source_stats,
                                    candidate.source.id.as_str(),
                                )
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
                            if let Some(stats) = source_stats.get_mut(candidate.source.id.as_str())
                            {
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
                                && let Some(progress) = advance_source_progress(
                                    &mut source_stats,
                                    candidate.source.id.as_str(),
                                )
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
                            if let Some(stats) = source_stats.get_mut(candidate.source.id.as_str())
                            {
                                stats.earliest_retry_at =
                                    earliest_deadline(stats.earliest_retry_at, Some(retry_at));
                            }
                            let aggregate = aggregate_source_stats(source_stats.values().copied());
                            next_retry_at = aggregate.earliest_retry_at;
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
            let aggregate = aggregate_source_stats(source_stats.values().copied());
            telemetry.readiness_queue_depth = aggregate.readiness_queue_depth;
            telemetry.readiness_queue_depth_by_source = source_stats
                .iter()
                .map(|(source_id, stats)| (source_id.clone(), stats.readiness_queue_depth))
                .collect();
            drop(telemetry);
            if last_similarity_refresh_publish_at.is_none_or(|published_at| {
                published_at.elapsed() >= SIMILARITY_SCORE_REFRESH_INTERVAL
            }) && publish_similarity_readiness_refreshes(
                &shared,
                &mut pending_similarity_refresh_lifecycles,
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
        if publish_similarity_readiness_refreshes(
            &shared,
            &mut pending_similarity_refresh_lifecycles,
        ) {
            last_similarity_refresh_publish_at = Some(Instant::now());
        }
        let active_source_has_runnable_work = scheduler.active_source().is_some_and(|source_id| {
            candidates
                .iter()
                .any(|candidate| candidate.source.id.as_str() == source_id)
        });
        if !active_source_has_runnable_work
            && publish_source_processing_wait(
                &shared,
                scheduler.active_source(),
                &source_lifecycle_generations,
                &source_stats,
            )
        {
            progress_visible = true;
            active_progress_source = None;
            last_progress_publish_at = Some(Instant::now());
        }
        let mut telemetry = shared.telemetry();
        telemetry.queue_depth = candidates.len();
        telemetry.oldest_job_age_seconds = oldest_job_age_seconds(&candidates, now_epoch_seconds());
        telemetry.queue_depth_by_source = queue_depths_by_source(&candidates);
        telemetry.settled_wake_generation = observed_generation;
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.sweep",
            reason,
            active_source_id = scheduler.active_source().unwrap_or(""),
            source_count = sources.len(),
            queued = telemetry.queue_depth,
            queue_depth_by_source = ?telemetry.queue_depth_by_source,
            readiness_queue_depth_by_source = ?telemetry.readiness_queue_depth_by_source,
            retries_due_by_source = ?telemetry.retries_due_by_source,
            retry_at_by_source = ?telemetry.retry_at_by_source,
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
        if scheduler.active_source().is_none()
            && !pending_discovery_sources.is_empty()
            && !discovery_deferred_for_capacity
        {
            shared.control().notify("next_source_discovery");
            shared.wake.notify_one();
        }
    }
}

fn run_retirement_worker(shared: Arc<Shared>) {
    loop {
        let control = shared.control();
        let (control, _) = shared
            .retirement_wake
            .wait_timeout_while(control, Duration::from_secs(1), |control| {
                !control.shutdown && !source_retirement_is_ready(control, now_epoch_seconds())
            })
            .unwrap_or_else(|poison| poison.into_inner());
        if control.shutdown {
            return;
        }
        if !source_retirement_is_ready(&control, now_epoch_seconds()) {
            continue;
        }
        drop(control);
        process_ready_source_retirements(shared.as_ref());
        let control = shared.control();
        drop(
            shared
                .retirement_wake
                .wait_timeout(control, Duration::from_millis(250))
                .unwrap_or_else(|poison| poison.into_inner()),
        );
    }
}

fn source_retirement_is_ready(control: &ControlState, now: i64) -> bool {
    control.pending_retirements.values().any(|retirement| {
        (!retirement.terminal_offline && retirement.retry_at <= now)
            || control
                .sources
                .values()
                .any(|active| source_storage_identity_matches(active, &retirement.source))
    })
}

fn clear_satisfied_manifest_audit_request(shared: &Shared, source_id: &str) {
    let mut control = shared.control();
    // A watcher-ready boundary or another reconciliation request can arrive
    // while this audit is in flight. That newer request remains in
    // `dirty_sources` until the coordinator snapshots it, so preserve the
    // force flag for the closing audit instead of letting the older completion
    // erase it.
    if !control.dirty_sources.contains(source_id) {
        control.force_manifest_audit_sources.remove(source_id);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateInvalidationScope {
    None,
    TargetScope,
    Source,
}

fn candidate_invalidation_scope(
    task: &RuntimeTask,
    outcome: Option<ExecutionOutcome>,
) -> CandidateInvalidationScope {
    match (task, outcome) {
        (
            RuntimeTask::ManifestAudit,
            Some(
                ExecutionOutcome::Completed
                | ExecutionOutcome::CompletedAwaitingForegroundRefresh
                | ExecutionOutcome::FailedAwaitingForegroundRefresh,
            ),
        ) => CandidateInvalidationScope::Source,
        (RuntimeTask::Readiness(target), Some(ExecutionOutcome::Completed))
            if target.stage == ReadinessStage::IndexedIdentity
                && target.content_generation.starts_with("pending-") =>
        {
            CandidateInvalidationScope::TargetScope
        }
        (
            RuntimeTask::Readiness(_),
            Some(
                ExecutionOutcome::Retried { .. }
                | ExecutionOutcome::Failed
                | ExecutionOutcome::PrerequisiteInvalidated { .. }
                | ExecutionOutcome::Stale
                | ExecutionOutcome::NotClaimed,
            ),
        ) => CandidateInvalidationScope::TargetScope,
        _ => CandidateInvalidationScope::None,
    }
}

fn process_ready_source_retirements(shared: &Shared) {
    let now = now_epoch_seconds();
    let candidates = {
        let control = shared.control();
        control
            .pending_retirements
            .iter()
            .filter(|(_, retirement)| {
                (!retirement.terminal_offline && retirement.retry_at <= now)
                    || control
                        .sources
                        .values()
                        .any(|active| source_storage_identity_matches(active, &retirement.source))
            })
            .map(|(retirement_id, retirement)| (*retirement_id, retirement.clone()))
            .collect::<Vec<_>>()
    };

    for (retirement_id, retirement) in candidates {
        // Fence only the admission snapshot and final publication. The potentially blocking
        // source I/O runs in a killable child without owning `source_replacement`, so a fast
        // re-add can cancel and supersede it instead of freezing source configuration.
        {
            let _replacement = shared
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
            if let Some(source_id) = reactivated_source_id(&control, &retirement.source) {
                control.pending_retirements.remove(&retirement_id);
                control.dirty_sources.insert(source_id);
                control.notify("source_storage_handoff_completed");
                drop(control);
                shared.wake.notify_all();
                shared.budget_wake.notify_all();
                continue;
            }
        }
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.retirement.started",
            source_id = retirement.source.id.as_str(),
            source_root = %retirement.source.root.display(),
            lifecycle_generation = retirement.lifecycle_generation,
            "Retiring removed source state"
        );
        #[cfg(test)]
        let result = if shared.retirement_cleanup_blocked.load(Ordering::Acquire) {
            shared
                .retirement_cleanup_started
                .store(true, Ordering::Release);
            while !shared.cancel.load(Ordering::Acquire)
                && !retirement.cancel.load(Ordering::Acquire)
            {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None)
        } else {
            super::worker::run_source_retirement(&retirement.source, retirement.cancel.as_ref())
        };
        #[cfg(not(test))]
        let result =
            super::worker::run_source_retirement(&retirement.source, retirement.cancel.as_ref());

        let _replacement = shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut control = shared.control();
        let Some(current) = control.pending_retirements.get(&retirement_id) else {
            continue;
        };
        if current.lifecycle_generation != retirement.lifecycle_generation {
            continue;
        }
        if let Some(source_id) = reactivated_source_id(&control, &retirement.source) {
            control.pending_retirements.remove(&retirement_id);
            control.dirty_sources.insert(source_id);
            control.notify("source_storage_handoff_completed");
            drop(control);
            shared.wake.notify_all();
            shared.budget_wake.notify_all();
            continue;
        }
        if control.shutdown {
            return;
        }
        match result {
            Ok(Some(SourceRetirementOutcome::Retired { retired_cache_refs })) => {
                control.pending_retirements.remove(&retirement_id);
                tracing::info!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.completed",
                    source_id = retirement.source.id.as_str(),
                    retired_cache_refs,
                    "Retired removed source runtime and path-derived cache ownership"
                );
            }
            Ok(Some(SourceRetirementOutcome::TerminalOffline)) => {
                if let Some(pending) = control.pending_retirements.get_mut(&retirement_id) {
                    pending.terminal_offline = true;
                    pending.retry_at = i64::MAX;
                }
                tracing::info!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.offline",
                    source_id = retirement.source.id.as_str(),
                    lifecycle_generation = retirement.lifecycle_generation,
                    "Removed source storage is offline; retirement is parked until re-add"
                );
            }
            Ok(None) => {
                control.pending_retirements.remove(&retirement_id);
                tracing::info!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.cancelled",
                    source_id = retirement.source.id.as_str(),
                    lifecycle_generation = retirement.lifecycle_generation,
                    "Cancelled superseded removed-source retirement"
                );
            }
            Err(error) => {
                if let Some(pending) = control.pending_retirements.get_mut(&retirement_id) {
                    pending.attempts = pending.attempts.saturating_add(1);
                    let delay = SOURCE_RETIREMENT_RETRY_SECONDS
                        .saturating_mul(1_i64 << pending.attempts.min(6));
                    pending.retry_at = now.saturating_add(delay);
                }
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.retry",
                    source_id = retirement.source.id.as_str(),
                    attempt = control
                        .pending_retirements
                        .get(&retirement_id)
                        .map_or(0, |pending| pending.attempts),
                    retry_at = control
                        .pending_retirements
                        .get(&retirement_id)
                        .map_or(0, |pending| pending.retry_at),
                    error,
                    "Removed source retirement will retry without reactivating the source"
                );
            }
        }
    }
}

fn reactivated_source_id(control: &ControlState, retired_source: &SampleSource) -> Option<String> {
    control.sources.values().find_map(|active| {
        source_storage_identity_matches(active, retired_source)
            .then(|| active.id.as_str().to_string())
    })
}

fn progress_refresh_due(last_publish_at: Option<Instant>) -> bool {
    last_publish_at.is_none_or(|published_at| published_at.elapsed() >= PROGRESS_REFRESH_INTERVAL)
}

fn manifest_audit_source_row_active(started_at: Instant) -> bool {
    started_at.elapsed() >= DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL
}

fn publish_similarity_readiness_refreshes(
    shared: &Shared,
    pending_lifecycles: &mut BTreeSet<SourceProcessingLifecycle>,
) -> bool {
    if pending_lifecycles.is_empty() {
        return false;
    }
    let mut published = false;
    for lifecycle in std::mem::take(pending_lifecycles) {
        published |=
            shared.publish_event(SourceProcessingEvent::SimilarityReadinessAdvanced { lifecycle });
    }
    published
}

fn publish_source_processing_progress(
    shared: &Shared,
    candidate: &RuntimeCandidate,
    lifecycle_generation: u64,
    stats: SourceDiscoveryStats,
) {
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
    let activity = match &candidate.task {
        RuntimeTask::Readiness(target) => SourceProcessingActivity::Readiness {
            stage: target.stage,
            relative_path: target.relative_path.clone(),
        },
        RuntimeTask::ManifestAudit => SourceProcessingActivity::ManifestAudit {
            checked: None,
            relative_path: None,
        },
    };
    shared.publish_event(SourceProcessingEvent::Progress(
        SourceProcessingProgressEvent {
            lifecycle: SourceProcessingLifecycle::new(
                candidate.source.id.as_str(),
                lifecycle_generation,
            ),
            source_row_active: !matches!(candidate.task, RuntimeTask::ManifestAudit),
            completed,
            total,
            activity,
        },
    ));
}

fn publish_source_processing_finished(shared: &Shared) {
    shared.publish_event(SourceProcessingEvent::Completed);
}

#[cfg(test)]
fn publish_source_processing_prerequisite_wait(
    shared: &Shared,
    lifecycle_generations: &BTreeMap<String, u64>,
    source_stats: &BTreeMap<String, SourceDiscoveryStats>,
) -> bool {
    let Some((source_id, stats)) = source_stats
        .iter()
        .filter(|(_, stats)| stats.prerequisites_blocked > 0)
        .min_by_key(|(_, stats)| stats.prerequisite_retry_at.unwrap_or(i64::MAX))
    else {
        return false;
    };
    publish_source_processing_wait_for_source(shared, source_id, lifecycle_generations, stats, true)
}

fn publish_source_processing_wait(
    shared: &Shared,
    active_source_id: Option<&str>,
    lifecycle_generations: &BTreeMap<String, u64>,
    source_stats: &BTreeMap<String, SourceDiscoveryStats>,
) -> bool {
    let Some(source_id) = active_source_id else {
        return false;
    };
    let Some(stats) = source_stats.get(source_id) else {
        return false;
    };
    if stats.prerequisites_blocked > 0 {
        return publish_source_processing_wait_for_source(
            shared,
            source_id,
            lifecycle_generations,
            stats,
            true,
        );
    }
    if stats.earliest_retry_at.is_none() {
        return false;
    }
    publish_source_processing_wait_for_source(
        shared,
        source_id,
        lifecycle_generations,
        stats,
        false,
    )
}

fn publish_source_processing_wait_for_source(
    shared: &Shared,
    source_id: &str,
    lifecycle_generations: &BTreeMap<String, u64>,
    stats: &SourceDiscoveryStats,
    prerequisite_wait: bool,
) -> bool {
    let Some(lifecycle_generation) = lifecycle_generations.get(source_id).copied() else {
        return false;
    };
    let control = shared.control();
    if !control.source_is_active(source_id)
        || control.source_lifecycle_generations.get(source_id) != Some(&lifecycle_generation)
    {
        return false;
    }
    drop(control);
    let retry_at = if prerequisite_wait {
        stats.prerequisite_retry_at
    } else {
        stats.earliest_retry_at
    };
    let activity = match (prerequisite_wait, retry_at) {
        (true, retry_at) => SourceProcessingActivity::WaitingForPrerequisites { retry_at },
        (false, Some(retry_at)) => SourceProcessingActivity::WaitingForRetry { retry_at },
        (false, None) => return false,
    };
    shared.publish_event(SourceProcessingEvent::Progress(
        SourceProcessingProgressEvent {
            lifecycle: SourceProcessingLifecycle::new(source_id, lifecycle_generation),
            source_row_active: true,
            completed: stats.progress_completed,
            total: stats.progress_total,
            activity,
        },
    ))
}

fn advance_source_progress(
    source_stats: &mut BTreeMap<String, SourceDiscoveryStats>,
    source_id: &str,
) -> Option<SourceDiscoveryStats> {
    let stats = source_stats.get_mut(source_id)?;
    stats.readiness_queue_depth = stats.readiness_queue_depth.saturating_sub(1);
    stats.progress_completed = stats
        .progress_completed
        .saturating_add(1)
        .min(stats.progress_total);
    Some(*stats)
}

fn similarity_prerequisite_blocker_stats(snapshot: &ReadinessSnapshot) -> (usize, Option<i64>) {
    let Some(layout) = snapshot.entries.iter().find(|entry| {
        entry.target.stage == ReadinessStage::SimilarityLayout
            && entry.target.eligibility == ReadinessEligibility::Eligible
            && entry.classification != ReadinessClassification::Current
            && !snapshot.prerequisites_are_current(&entry.target)
    }) else {
        return (0, None);
    };
    let mut blocked = 0_usize;
    let mut all_retryable = true;
    let mut earliest_retry_at = None;
    for entry in snapshot.entries.iter().filter(|entry| {
        entry.target.source_id == layout.target.source_id
            && entry.target.source_generation == layout.target.source_generation
            && entry.target.scope_kind == ReadinessScopeKind::File
            && entry.target.eligibility == ReadinessEligibility::Eligible
            && matches!(
                entry.target.stage,
                ReadinessStage::IndexedIdentity
                    | ReadinessStage::AnalysisFeatures
                    | ReadinessStage::EmbeddingAspects
            )
            && entry.classification != ReadinessClassification::Current
    }) {
        blocked = blocked.saturating_add(1);
        match entry.classification {
            ReadinessClassification::RetryableFailure { retry_at, .. } => {
                earliest_retry_at = earliest_deadline(earliest_retry_at, Some(retry_at));
            }
            _ => all_retryable = false,
        }
    }
    (
        blocked,
        all_retryable.then_some(earliest_retry_at).flatten(),
    )
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
    force_manifest_audit_sources: &BTreeSet<String>,
    force_reanalysis_sources: &BTreeSet<String>,
    pending_readiness_deltas: &BTreeMap<String, PendingReadinessDelta>,
    safety_probe_only: bool,
    source_work_cancels: &BTreeMap<String, Arc<AtomicBool>>,
) -> (
    Vec<RuntimeCandidate>,
    BTreeMap<String, SourceDiscoveryStats>,
    BTreeSet<String>,
    BTreeSet<String>,
    bool,
) {
    let now = now_epoch_seconds();
    let mut candidates = Vec::new();
    let mut source_stats = BTreeMap::new();
    let mut deferred = BTreeSet::new();
    let mut consumed_readiness_deltas = BTreeSet::new();
    let mut progress_published = false;
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
        let mut progress = DiscoveryProgressPublisher {
            shared,
            source_id: source.id.as_str(),
            lifecycle_generation: in_flight_work.lifecycle_generation,
            started_at: Instant::now(),
            last_phase: None,
            last_event_publish_at: None,
            last_log_publish_at: None,
            event_published: false,
        };
        match discover_source_candidates_with_progress(
            source,
            now,
            force_manifest_audit_sources.contains(source.id.as_str()),
            force_reanalysis_sources.contains(source.id.as_str()),
            pending_readiness_deltas.get(source.id.as_str()),
            safety_probe_only,
            source_cancel,
            &mut |phase, work_units| progress.advance(phase, work_units),
        ) {
            Ok(Cancellable::Completed((mut source_candidates, stats))) => {
                if stats.cheap_noop_sweep {
                    let mut telemetry = shared.telemetry();
                    telemetry.cheap_noop_sweeps = telemetry.cheap_noop_sweeps.saturating_add(1);
                }
                if stats.delta_reconciled {
                    let mut telemetry = shared.telemetry();
                    telemetry.delta_reconciliations =
                        telemetry.delta_reconciliations.saturating_add(1);
                }
                candidates.append(&mut source_candidates);
                if !stats.cheap_noop_sweep {
                    source_stats.insert(source.id.as_str().to_string(), stats);
                }
                if pending_readiness_deltas.contains_key(source.id.as_str()) {
                    consumed_readiness_deltas.insert(source.id.as_str().to_string());
                }
                shared
                    .control()
                    .force_reanalysis_sources
                    .remove(source.id.as_str());
            }
            Ok(Cancellable::Cancelled) => {
                deferred.insert(source.id.as_str().to_string());
            }
            Err(error) => {
                record_discovery_error(shared, source, &error);
                source_stats.insert(
                    source.id.as_str().to_string(),
                    SourceDiscoveryStats {
                        earliest_retry_at: Some(
                            now_epoch_seconds().saturating_add(SOURCE_DISCOVERY_RETRY_SECONDS),
                        ),
                        ..SourceDiscoveryStats::default()
                    },
                );
            }
        }
        progress_published |= progress.event_published;
        drop(in_flight_work);
        shared.budgets().release(permit);
        shared.budget_wake.notify_all();
    }
    (
        candidates,
        source_stats,
        deferred,
        consumed_readiness_deltas,
        progress_published,
    )
}

#[cfg(test)]
fn discover_source_candidates(
    source: &SampleSource,
    now: i64,
    force_manifest_audit: bool,
    cancel: &AtomicBool,
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    discover_source_candidates_with_progress(
        source,
        now,
        force_manifest_audit,
        false,
        None,
        false,
        cancel,
        &mut |_, _| {},
    )
}

fn discover_source_candidates_with_progress(
    source: &SampleSource,
    now: i64,
    force_manifest_audit: bool,
    force_reanalysis: bool,
    pending_readiness_delta: Option<&PendingReadinessDelta>,
    safety_probe_only: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(&'static str, usize),
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    if !source.root.is_dir() {
        if database_root != source.root && database_root.is_dir() {
            let mut connection = SourceDatabase::open_unavailable_source_metadata_connection(
                &database_root,
                SourceDatabaseConnectionRole::JobWorker,
            )
            .map_err(|error| error.to_string())?;
            if source_processing_schema_available(&mut connection)? {
                ReadinessStore::new(&mut connection)
                    .mark_temporarily_unavailable(source.id.as_str(), now)
                    .map_err(|error| error.to_string())?;
            }
        }
        return Ok(Cancellable::Completed((
            Vec::new(),
            SourceDiscoveryStats::default(),
        )));
    }
    if safety_probe_only {
        match SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::BackgroundRead,
        ) {
            Ok(mut probe_connection) => {
                if readiness_safety_probe_is_current(
                    &mut probe_connection,
                    source.id.as_str(),
                    now,
                    force_manifest_audit,
                )? {
                    tracing::debug!(
                        target: "wavecrate::source_processing",
                        event = "source_processing.safety_sweep_noop",
                        source_id = source.id.as_str(),
                        "Periodic readiness safety probe found no durable delta"
                    );
                    return Ok(Cancellable::Completed((
                        Vec::new(),
                        SourceDiscoveryStats {
                            cheap_noop_sweep: true,
                            ..SourceDiscoveryStats::default()
                        },
                    )));
                }
            }
            Err(error) => {
                tracing::debug!(
                    target: "wavecrate::source_processing",
                    source_id = source.id.as_str(),
                    %error,
                    "Read-only readiness safety probe unavailable; retrying with worker connection"
                );
            }
        }
    }
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    discover_source_candidates_with_connection_and_progress(
        source,
        &mut connection,
        now,
        force_manifest_audit,
        force_reanalysis,
        pending_readiness_delta,
        false,
        cancel,
        progress,
    )
}

fn readiness_safety_probe_is_current(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    force_manifest_audit: bool,
) -> Result<bool, String> {
    if force_manifest_audit || !source_processing_schema_available(connection)? {
        return Ok(false);
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
    if now.saturating_sub(last_manifest_audit_at) >= MANIFEST_AUDIT_INTERVAL_SECONDS {
        return Ok(false);
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
    let contract_version = readiness_contract_version();
    Ok(ReadinessStore::new(connection)
        .source_state(source_id)
        .map_err(|error| error.to_string())?
        .is_some_and(|state| {
            state.source_generation == source_generation
                && state.availability == SourceAvailability::Active
                && state.contract_version == contract_version
        }))
}

#[cfg(test)]
fn discover_source_candidates_with_connection(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    now: i64,
    force_manifest_audit: bool,
    cancel: &AtomicBool,
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    discover_source_candidates_with_connection_and_progress(
        source,
        connection,
        now,
        force_manifest_audit,
        false,
        None,
        false,
        cancel,
        &mut |_, _| {},
    )
}

fn discover_source_candidates_with_connection_and_progress(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    now: i64,
    force_manifest_audit: bool,
    force_reanalysis: bool,
    pending_readiness_delta: Option<&PendingReadinessDelta>,
    safety_probe_only: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(&'static str, usize),
) -> Result<Cancellable<(Vec<RuntimeCandidate>, SourceDiscoveryStats)>, String> {
    let source_id = source.id.as_str();
    let mut work_units = 0_usize;
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
    if !source_processing_schema_available(connection)? {
        tracing::debug!(
            target: "wavecrate::source_processing",
            source_id,
            "Source processing is unavailable until the read-only source database is migrated"
        );
        return Ok(Cancellable::Completed((candidates, stats)));
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
    if safety_probe_only
        && readiness_safety_probe_is_current(connection, source_id, now, force_manifest_audit)?
    {
        stats.cheap_noop_sweep = true;
        tracing::debug!(
            target: "wavecrate::source_processing",
            event = "source_processing.safety_sweep_noop",
            source_id,
            "Periodic readiness safety probe found no durable delta"
        );
        return Ok(Cancellable::Completed((candidates, stats)));
    }
    if force_reanalysis {
        let changed = ReadinessStore::new(connection)
            .requeue_source_analysis(source_id, now)
            .map_err(|error| format!("Requeue source analysis failed: {error}"))?;
        tracing::info!(
            target: "wavecrate::source_processing",
            source_id,
            changed,
            "Requeued source analysis through readiness"
        );
    }
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
    if force_manifest_audit
        || manifest_identity_repair_due
        || now.saturating_sub(last_manifest_audit_at) >= MANIFEST_AUDIT_INTERVAL_SECONDS
    {
        candidates.push(RuntimeCandidate {
            schedule: WorkCandidate::source(source_id, ProcessingLane::Scan, 0, now),
            source: source.clone(),
            task: RuntimeTask::ManifestAudit,
        });
    }
    progress("Retiring legacy playback readiness", work_units);
    if matches!(
        retire_legacy_playback_readiness(source, connection, cancel)?,
        Cancellable::Cancelled
    ) {
        return Ok(Cancellable::Cancelled);
    }
    let mut delta_applied = false;
    if let Some(delta) = pending_readiness_delta.filter(|delta| !delta.is_empty()) {
        match publish_current_readiness_delta_with_cancel(
            connection, source_id, delta, now, cancel,
        )? {
            Cancellable::Completed(Some(_changed)) => {
                stats.delta_reconciled = true;
                delta_applied = true;
            }
            Cancellable::Completed(None) => {}
            Cancellable::Cancelled => return Ok(Cancellable::Cancelled),
        }
    }
    if !delta_applied {
        let target_publication = publish_current_readiness_targets_with_cancel_and_checkpoint(
            connection,
            source_id,
            now,
            cancel,
            true,
            &mut || {
                work_units = work_units.saturating_add(1);
                progress("Reading manifest and readiness targets", work_units);
            },
        )?;
        if matches!(target_publication, Cancellable::Cancelled) {
            return Ok(Cancellable::Cancelled);
        }
    }
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let readiness_source_exists = ReadinessStore::new(connection)
        .source_exists(source_id)
        .map_err(|error| error.to_string())?;
    if readiness_source_exists {
        let reclassified = ReadinessStore::new(connection)
            .reclassify_known_unsupported_failures(legacy_unsupported_decode_failure_text)
            .map_err(|error| error.to_string())?;
        if reclassified > 0 {
            tracing::info!(
                target: "wavecrate::source_processing",
                source_id,
                reclassified,
                "Reclassified deterministic audio decode failures as unsupported"
            );
        }
        let reconciliation = if delta_applied {
            ReadinessStore::new(connection).reconcile_scopes_with_cancel_and_progress(
                source_id,
                &pending_readiness_delta
                    .expect("a published readiness delta has affected scopes")
                    .scope_ids,
                now,
                cancel,
                &mut || {
                    work_units = work_units.saturating_add(1);
                    progress("Comparing changed readiness", work_units);
                },
            )
        } else {
            ReadinessStore::new(connection).reconcile_with_cancel_and_progress(
                source_id,
                now,
                cancel,
                &mut || {
                    work_units = work_units.saturating_add(1);
                    progress("Comparing durable readiness", work_units);
                },
            )
        };
        let snapshot = match reconciliation {
            Ok(snapshot) => snapshot,
            Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
                return Ok(Cancellable::Cancelled);
            }
            Err(error) => return Err(error.to_string()),
        };
        let active_recordings = active_recording_deferrals(connection, now)?;
        let persistable_deficits = snapshot
            .deficits
            .iter()
            .filter(|deficit| {
                !active_recordings
                    .scope_ids
                    .contains(&deficit.target.scope_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        match ReadinessStore::new(connection).persist_deficits_with_cancel_and_progress(
            &persistable_deficits,
            now,
            cancel,
            &mut || {
                work_units = work_units.saturating_add(1);
                progress("Queueing unfinished jobs", work_units);
            },
        ) {
            Ok(_) => {}
            Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
                return Ok(Cancellable::Cancelled);
            }
            Err(error) => return Err(error.to_string()),
        }
        ReadinessStore::new(connection)
            .defer_active_recordings(&active_recordings.scope_ids)
            .map_err(|error| error.to_string())?;
        let schedulable_deficits = persistable_deficits
            .iter()
            .filter(|deficit| delta_applied || snapshot.prerequisites_are_current(&deficit.target))
            .collect::<Vec<_>>();
        stats.readiness_queue_depth = schedulable_deficits.len();
        (stats.prerequisites_blocked, stats.prerequisite_retry_at) =
            similarity_prerequisite_blocker_stats(&snapshot);
        candidates.extend(schedulable_deficits.iter().map(|deficit| RuntimeCandidate {
            schedule: WorkCandidate::readiness(&deficit.target, deficit.enqueued_at.unwrap_or(now)),
            source: source.clone(),
            task: RuntimeTask::Readiness(deficit.target.clone()),
        }));
        let work_stats = if delta_applied {
            None
        } else {
            Some(
                ReadinessStore::new(connection)
                    .work_stats(now)
                    .map_err(|error| error.to_string())?,
            )
        };
        if let Some(work_stats) = work_stats {
            stats.progress_total = work_stats.total;
            stats.progress_completed = work_stats
                .completed
                .saturating_add(work_stats.permanent_failures)
                .saturating_add(work_stats.unsupported)
                .min(stats.progress_total);
            stats.retries_due = work_stats.retries_due;
            stats.earliest_retry_at = earliest_deadline(
                earliest_deadline(
                    work_stats.earliest_retry_at,
                    work_stats.earliest_lease_expiry_at,
                ),
                active_recordings.retry_at,
            );
            tracing::debug!(
                target: "wavecrate::source_processing",
                source_id,
                pending = work_stats.pending,
                running = work_stats.running,
                retries_due = work_stats.retries_due,
                retries_waiting = work_stats.retries_waiting,
                expired_leases = work_stats.expired_leases,
                prerequisites_blocked = stats.prerequisites_blocked,
                "Readiness work reconciled"
            );
        } else {
            stats.progress_total = snapshot.entries.len();
            stats.progress_completed = snapshot
                .entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.classification,
                        ReadinessClassification::Current
                            | ReadinessClassification::PermanentFailure { .. }
                            | ReadinessClassification::Unsupported
                    )
                })
                .count();
            for entry in &snapshot.entries {
                match entry.classification {
                    ReadinessClassification::RetryableFailure { retry_at, .. } => {
                        stats.earliest_retry_at =
                            earliest_deadline(stats.earliest_retry_at, Some(retry_at));
                        if retry_at <= now {
                            stats.retries_due = stats.retries_due.saturating_add(1);
                        }
                    }
                    ReadinessClassification::Running { lease_expires_at } => {
                        stats.earliest_retry_at =
                            earliest_deadline(stats.earliest_retry_at, Some(lease_expires_at));
                    }
                    _ => {}
                }
            }
            stats.earliest_retry_at =
                earliest_deadline(stats.earliest_retry_at, active_recordings.retry_at);
        }
        if !active_recordings.scope_ids.is_empty() {
            tracing::info!(
                target: "wavecrate::source_processing",
                event = "source_processing.active_recordings_deferred",
                source_id,
                file_count = active_recordings.scope_ids.len(),
                retry_at = active_recordings.retry_at.unwrap_or_default(),
                "Deferred files that are still being actively written"
            );
        }
    }

    if cancelled(cancel) {
        Ok(Cancellable::Cancelled)
    } else {
        Ok(Cancellable::Completed((candidates, stats)))
    }
}

#[derive(Debug, Default)]
struct ActiveRecordingDeferrals {
    scope_ids: BTreeSet<String>,
    retry_at: Option<i64>,
}

fn active_recording_deferrals(
    connection: &rusqlite::Connection,
    now: i64,
) -> Result<ActiveRecordingDeferrals, String> {
    const NANOS_PER_SECOND: i64 = 1_000_000_000;
    let end_of_current_second_ns = now
        .saturating_add(1)
        .saturating_mul(NANOS_PER_SECOND)
        .saturating_sub(1);
    let cutoff_ns = now
        .saturating_sub(ACTIVE_RECORDING_QUIET_SECONDS)
        .saturating_mul(NANOS_PER_SECOND);
    let mut statement = connection
        .prepare(
            "SELECT file_identity, modified_ns
             FROM wav_files
             WHERE missing = 0
               AND file_identity IS NOT NULL
               AND TRIM(file_identity) != ''
               AND modified_ns BETWEEN ?1 AND ?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![cutoff_ns, end_of_current_second_ns], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut deferrals = ActiveRecordingDeferrals::default();
    for row in rows {
        let (scope_id, modified_ns) = row.map_err(|error| error.to_string())?;
        deferrals.scope_ids.insert(scope_id);
        let modified_second = modified_ns.div_euclid(NANOS_PER_SECOND);
        let stable_at = modified_second
            .saturating_add(ACTIVE_RECORDING_QUIET_SECONDS)
            .saturating_add(1);
        deferrals.retry_at = earliest_deadline(deferrals.retry_at, Some(stable_at));
    }
    Ok(deferrals)
}

fn source_processing_schema_available(
    connection: &mut rusqlite::Connection,
) -> Result<bool, String> {
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
    ReadinessStore::new(connection)
        .processing_schema_available()
        .map_err(|error| error.to_string())
}

pub(super) enum SourceRetirementOutcome {
    Retired { retired_cache_refs: usize },
    TerminalOffline,
}

pub(super) fn retire_source_derived_state(
    source: &SampleSource,
) -> Result<SourceRetirementOutcome, String> {
    let database_path = source.db_path().map_err(|error| error.to_string())?;
    if !database_path.exists() {
        if source.metadata_storage == SourceMetadataStorage::SourceFolder && !source.root.is_dir() {
            return Ok(SourceRetirementOutcome::TerminalOffline);
        }
        return Ok(SourceRetirementOutcome::Retired {
            retired_cache_refs: 0,
        });
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
    if !ReadinessStore::new(&mut connection)
        .schema_available()
        .map_err(|error| error.to_string())?
    {
        return Ok(SourceRetirementOutcome::Retired {
            retired_cache_refs: 0,
        });
    }
    let cleanup = ReadinessStore::new(&mut connection)
        .retire_source(source.id.as_str(), now_epoch_seconds())
        .map_err(|error| error.to_string())?;
    let mut invalidated = 0;
    for cache_ref in &cleanup.retired_artifact_refs {
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
    if let Err(error) = prune_unreferenced_waveform_cache() {
        tracing::warn!(
            target: "wavecrate::source_processing",
            source_id = source.id.as_str(),
            error,
            "Bounded orphan cache collection was deferred"
        );
    }
    Ok(SourceRetirementOutcome::Retired {
        retired_cache_refs: invalidated,
    })
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
        let mut connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| error.to_string())?;
        let owned = ReadinessStore::new(&mut connection)
            .legacy_playback_artifact_ref_is_owned(cache_ref)
            .map_err(|error| error.to_string())?;
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
        let mut connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| format!("open retained cache manifest: {error}"))?;
        let refs = ReadinessStore::new(&mut connection)
            .legacy_playback_artifact_refs()
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

#[cfg(test)]
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
        false,
        &mut || {},
    )
}

fn publish_current_readiness_delta_with_cancel(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    delta: &PendingReadinessDelta,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<Option<usize>>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
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
    let contract_version = readiness_contract_version();
    let Some(current_source) = ReadinessStore::new(connection)
        .source_state(source_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Cancellable::Completed(None));
    };
    if current_source.contract_version != contract_version {
        return Ok(Cancellable::Completed(None));
    }
    if current_source.source_generation == source_generation {
        return Ok(Cancellable::Completed(Some(0)));
    }

    let embedding_version = format!(
        "{}+{}",
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    );
    let mut targets = Vec::with_capacity(delta.scope_ids.len().saturating_mul(3));
    let mut deleted_scope_ids = Vec::new();
    for scope_id in &delta.scope_ids {
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        let mut statement = connection
            .prepare(
                "SELECT path, content_hash, file_size, modified_ns
                 FROM wav_files
                 WHERE missing = 0 AND file_identity = ?1
                 ORDER BY path
                 LIMIT 2",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([scope_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        drop(statement);
        match rows.as_slice() {
            [] => deleted_scope_ids.push(scope_id.clone()),
            [(path, content_hash, file_size, modified_ns)] => {
                if !wavecrate_library::sample_sources::is_supported_audio(std::path::Path::new(
                    path,
                )) {
                    return Ok(Cancellable::Completed(None));
                }
                let content_generation = content_hash
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("pending-{scope_id}-{file_size}-{modified_ns}"));
                let unsupported = *file_size <= 0
                    || ReadinessStore::new(connection)
                        .generation_is_known_unsupported(source_id, scope_id, &content_generation)
                        .map_err(|error| error.to_string())?;
                targets.extend(file_readiness_targets(
                    source_id,
                    scope_id,
                    path,
                    source_generation,
                    &content_generation,
                    embedding_version.as_str(),
                    unsupported,
                ));
            }
            _ => return Ok(Cancellable::Completed(None)),
        }
    }
    let readiness_revision = current_source.readiness_revision.saturating_add(1);
    let similarity_artifact_version = native_similarity_artifact_version();
    let publication = ReadinessTargetDeltaPublication::new(
        source_id,
        source_generation,
        readiness_revision,
        SourceAvailability::Active,
        contract_version.as_str(),
        &targets,
        &deleted_scope_ids,
        similarity_artifact_version.as_str(),
        now,
    );
    let outcome = match ReadinessStore::new(connection)
        .publish_target_delta_with_cancel(&publication, cancel)
    {
        Ok(outcome) => outcome,
        Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
            return Ok(Cancellable::Cancelled);
        }
        Err(error) => return Err(error.to_string()),
    };
    let ReadinessDeltaPublicationOutcome::Applied {
        membership_generation,
        changed,
    } = outcome
    else {
        return Ok(Cancellable::Completed(None));
    };
    let similarity_state = serde_json::json!({
        "state": "dirty",
        "source_generation": source_generation,
        "membership_generation": membership_generation,
        "artifact_version": similarity_artifact_version,
    })
    .to_string();
    wavecrate_analysis::ann_index::mark_artifacts_dirty(connection, &similarity_state)?;
    tracing::debug!(
        target: "wavecrate::source_processing",
        event = "source_processing.readiness_delta_reconciled",
        source_id,
        source_generation,
        identities = delta.scope_ids.len(),
        target_upserts = targets.len(),
        target_deletes = deleted_scope_ids.len(),
        changed,
        "Applied committed readiness target delta"
    );
    Ok(Cancellable::Completed(Some(changed)))
}

fn file_readiness_targets(
    source_id: &str,
    identity: &str,
    path: &str,
    source_generation: i64,
    content_generation: &str,
    embedding_version: &str,
    unsupported: bool,
) -> [ReadinessTarget; 3] {
    let indexed = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::IndexedIdentity,
        READINESS_MANIFEST_VERSION,
        source_generation,
        content_generation,
    );
    let analysis = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::AnalysisFeatures,
        wavecrate_analysis::analysis_version(),
        source_generation,
        content_generation,
    );
    let embedding = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::EmbeddingAspects,
        embedding_version,
        source_generation,
        content_generation,
    );
    if unsupported {
        [
            indexed,
            analysis.with_eligibility(ReadinessEligibility::Unsupported),
            embedding.with_eligibility(ReadinessEligibility::Unsupported),
        ]
    } else {
        [indexed, analysis, embedding]
    }
}

fn publish_current_readiness_targets_with_cancel_and_checkpoint(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
    allow_revision_noop: bool,
    checkpoint: &mut impl FnMut(),
) -> Result<Cancellable<bool>, String> {
    checkpoint();
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
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
    let contract_version = readiness_contract_version();
    let current_source = ReadinessStore::new(connection)
        .source_state(source_id)
        .map_err(|error| error.to_string())?;
    if allow_revision_noop
        && current_source.as_ref().is_some_and(|state| {
            state.source_generation == source_generation
                && state.availability == SourceAvailability::Active
                && state.contract_version == contract_version
        })
    {
        return Ok(Cancellable::Completed(false));
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
    let unsupported_generations = ReadinessStore::new(connection)
        .unsupported_content_generations(source_id)
        .map_err(|error| error.to_string())?;
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
            ReadinessStore::new(connection)
                .mark_temporarily_unavailable(source_id, now)
                .map_err(|error| error.to_string())?;
            return Ok(Cancellable::Completed(false));
        };
        let content_hash = content_hash.filter(|value| !value.trim().is_empty());
        let content_generation = content_hash
            .clone()
            .unwrap_or_else(|| format!("pending-{identity}-{file_size}-{modified_ns}"));
        manifest.push((path, identity, content_hash, content_generation, file_size));
    }
    let embedding_version = format!(
        "{}+{}",
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    );
    let similarity_artifact_version = native_similarity_artifact_version();
    let mut membership = ReadinessMembership::default();
    let mut targets = Vec::with_capacity(manifest.len().saturating_mul(3).saturating_add(1));
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
            membership.add(identity, content_generation);
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
    let membership_generation = membership.generation();
    targets.push(ReadinessTarget::source(
        source_id,
        ReadinessStage::SimilarityLayout,
        &similarity_artifact_version,
        source_generation,
        membership_generation.as_str(),
    ));
    let readiness_revision = current_source
        .map(|state| state.readiness_revision.saturating_add(1))
        .unwrap_or(1);
    let publication = ReadinessTargetPublication::new(
        source_id,
        source_generation,
        readiness_revision,
        SourceAvailability::Active,
        contract_version.as_str(),
        &targets,
        now,
    );
    match ReadinessStore::new(connection).publish_targets_with_cancel(&publication, cancel) {
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
    Ok(Cancellable::Completed(true))
}

/// Retire rows written by the removed durable playback-summary readiness stage.
///
/// Current waveform and playback caches are managed by the independent cache lifecycle. This
/// compatibility pass exists only so writable source databases from older builds cannot keep stale
/// readiness work or reverse-ownership rows alive. Read-only reconciliation filters the same legacy
/// stage without mutating it.
fn retire_legacy_playback_readiness(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    cancel: &AtomicBool,
) -> Result<Cancellable<usize>, String> {
    retire_legacy_playback_readiness_with_post_commit_hook(source, connection, cancel, || {})
}

fn retire_legacy_playback_readiness_with_post_commit_hook(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    cancel: &AtomicBool,
    post_commit: impl FnOnce(),
) -> Result<Cancellable<usize>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let cleanup = ReadinessStore::new(connection)
        .retire_legacy_playback(source.id.as_str())
        .map_err(|error| error.to_string())?;
    post_commit();

    for cache_ref in cleanup.retired_artifact_refs {
        match retained_waveform_cache_ref_is_owned(&cache_ref) {
            Ok(false) => invalidate_persisted_waveform_cache_ref(std::path::Path::new(&cache_ref)),
            Ok(true) => {}
            Err(error) => tracing::warn!(
                target: "wavecrate::source_processing",
                source_id = source.id.as_str(),
                cache_ref,
                error,
                "Legacy playback cache ownership could not be proven; payload was preserved"
            ),
        }
    }
    Ok(Cancellable::Completed(cleanup.changed))
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Acquire)
}

fn readiness_contract_version() -> String {
    let mut hash = blake3::Hasher::new();
    let similarity_artifact_version = native_similarity_artifact_version();
    for component in [
        READINESS_MANIFEST_VERSION,
        wavecrate_analysis::analysis_version(),
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
        similarity_artifact_version.as_str(),
        READINESS_MEMBERSHIP_VERSION,
    ] {
        hash.update(component.as_bytes());
        hash.update(&[0]);
    }
    format!("readiness-contract-v2:{}", hash.finalize().to_hex())
}

fn execute_candidate(
    candidate: &RuntimeCandidate,
    lifecycle_generation: u64,
    cancel: &AtomicBool,
    publish_event: &mut dyn FnMut(SourceProcessingEvent) -> bool,
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
            let audit_started_at = Instant::now();
            let mut last_progress_publish_at = None::<Instant>;
            let mut publish_progress = |checked: usize, path: &std::path::Path| {
                let publish_due = last_progress_publish_at.is_none_or(|published_at| {
                    published_at.elapsed() >= Duration::from_millis(250)
                });
                if !publish_due {
                    return;
                }
                let relative = path.strip_prefix(&source_root).unwrap_or(path);
                let total = expected_files.max(checked);
                publish_event(SourceProcessingEvent::Progress(
                    SourceProcessingProgressEvent {
                        lifecycle: SourceProcessingLifecycle::new(
                            source_id.clone(),
                            lifecycle_generation,
                        ),
                        source_row_active: manifest_audit_source_row_active(audit_started_at),
                        completed: checked.min(total),
                        total,
                        activity: SourceProcessingActivity::ManifestAudit {
                            checked: Some(checked),
                            relative_path: Some(relative.to_path_buf()),
                        },
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
            let browser_refresh_required = !outcome.committed_delta.is_empty()
                && crate::native_app::source_processing::manifest_delta_requires_browser_refresh(
                    &outcome.committed_delta,
                );
            let audit_published = !outcome.committed_delta.is_empty()
                && publish_event(SourceProcessingEvent::ManifestAuditCommitted {
                    lifecycle: SourceProcessingLifecycle::new(
                        candidate.source.id.as_str(),
                        lifecycle_generation,
                    ),
                    committed_delta: outcome.committed_delta,
                });
            let foreground_refresh_owns_reconciliation =
                browser_refresh_required && audit_published;
            let incomplete = incomplete_error.is_some();
            if let Some(error) = incomplete_error {
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    source_id = candidate.source.id.as_str(),
                    error,
                    "Manifest audit published a committed checkpoint and remains due"
                );
            }
            Ok(manifest_audit_execution_outcome(
                foreground_refresh_owns_reconciliation,
                incomplete,
                cancel.load(Ordering::Acquire),
            ))
        }
        RuntimeTask::Readiness(target) => {
            execute_readiness_target(&candidate.source, target, cancel)
        }
    };
    if matches!(
        result,
        Ok(ExecutionOutcome::CompletedAwaitingForegroundRefresh
            | ExecutionOutcome::FailedAwaitingForegroundRefresh)
    ) {
        result
    } else if cancel.load(Ordering::Acquire) {
        Ok(ExecutionOutcome::Cancelled)
    } else {
        result
    }
}

fn manifest_audit_execution_outcome(
    foreground_refresh_owns_reconciliation: bool,
    incomplete: bool,
    cancelled: bool,
) -> ExecutionOutcome {
    match (
        foreground_refresh_owns_reconciliation,
        incomplete,
        cancelled,
    ) {
        (true, false, _) => ExecutionOutcome::CompletedAwaitingForegroundRefresh,
        (true, true, _) => ExecutionOutcome::FailedAwaitingForegroundRefresh,
        (false, _, true) => ExecutionOutcome::Cancelled,
        (false, false, false) => ExecutionOutcome::Completed,
        (false, true, false) => ExecutionOutcome::Failed,
    }
}

#[cfg(test)]
fn execute_synthetic_candidate_for_profile(
    candidate: &RuntimeCandidate,
    cancel: &AtomicBool,
    connections: &mut BTreeMap<String, rusqlite::Connection>,
) -> Result<ExecutionOutcome, String> {
    let RuntimeTask::Readiness(target) = &candidate.task else {
        return Ok(ExecutionOutcome::Completed);
    };
    let source_id = candidate.source.id.as_str();
    if !connections.contains_key(source_id) {
        let database_root = candidate
            .source
            .database_root()
            .map_err(|error| error.to_string())?;
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &candidate.source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .map_err(|error| error.to_string())?;
        connections.insert(source_id.to_string(), connection);
    }
    let connection = connections
        .get_mut(source_id)
        .expect("synthetic source connection was inserted");
    let now = now_epoch_seconds();
    let Some(claim) = ReadinessStore::new(connection)
        .claim(target, now, READINESS_LEASE_SECONDS)
        .map_err(|error| error.to_string())?
    else {
        return Ok(ExecutionOutcome::NotClaimed);
    };
    if cancel.load(Ordering::Acquire) {
        return cancel_claim(connection, &claim, "profile cancellation", now);
    }
    match ReadinessStore::new(connection)
        .complete(&claim, now_epoch_seconds())
        .map_err(|error| error.to_string())?
    {
        ArtifactPublishOutcome::Recorded => Ok(ExecutionOutcome::Completed),
        ArtifactPublishOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
    }
}

enum ReadinessExecutionOutcome {
    Complete(Option<std::path::PathBuf>),
    Retry(&'static str),
    Permanent(&'static str),
    Unsupported(&'static str),
    PrerequisiteInvalidated(&'static str),
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
    let Some(claim) = ReadinessStore::new(&mut connection)
        .claim(target, now, READINESS_LEASE_SECONDS)
        .map_err(|error| error.to_string())?
    else {
        return Ok(ExecutionOutcome::NotClaimed);
    };
    tracing::info!(
        target: "wavecrate::source_processing",
        event = "source_processing.readiness.claimed",
        source_id = source.id.as_str(),
        stage = ?target.stage,
        scope_id = target.scope_id.as_str(),
        claim_generation = claim.claim_generation(),
        claim_origin = claim.origin().as_str(),
        lease_expires_at = claim.lease_expires_at(),
        "Readiness work claimed"
    );
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
            let _ = ReadinessStore::new(&mut connection).cancel(
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
                Some(artifact_ref) => ReadinessStore::new(&mut connection)
                    .complete_with_artifact_ref(
                        &claim,
                        now_epoch_seconds(),
                        &artifact_ref.to_string_lossy(),
                    ),
                None => ReadinessStore::new(&mut connection).complete(&claim, now_epoch_seconds()),
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
            let outcome = ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    ReadinessFailureClassification::Retryable,
                    "readiness_retry",
                    reason,
                    now_epoch_seconds(),
                    policy,
                )
                .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
        Err(failure) => {
            let policy = ReadinessRetryPolicy::new(5, 5 * 60, READINESS_MAX_ATTEMPTS)
                .expect("valid readiness retry policy");
            let outcome = ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    failure.readiness_failure_classification(),
                    failure.code.as_str(),
                    &failure.context,
                    now_epoch_seconds(),
                    policy,
                )
                .map_err(|error| error.to_string())?;
            tracing::warn!(
                target: "wavecrate::source_processing",
                source_id = source.id.as_str(),
                failure_code = failure.code.as_str(),
                source_error = ?failure.source_error,
                "Readiness execution failed"
            );
            Ok(execution_outcome_for_failure(outcome))
        }
        Ok(ReadinessExecutionOutcome::Permanent(reason)) => {
            let policy =
                ReadinessRetryPolicy::new(5, 5 * 60, 1).expect("valid readiness terminal policy");
            let outcome = ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    ReadinessFailureClassification::Permanent,
                    "readiness_permanent",
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
            let outcome = ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    ReadinessFailureClassification::Unsupported,
                    "readiness_unsupported",
                    reason,
                    now_epoch_seconds(),
                    policy,
                )
                .map_err(|error| error.to_string())?;
            Ok(execution_outcome_for_failure(outcome))
        }
        Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated(reason)) => {
            let policy = ReadinessRetryPolicy::new(
                PREREQUISITE_INVALIDATION_RETRY_SECONDS,
                5 * 60,
                READINESS_MAX_ATTEMPTS,
            )
            .expect("valid prerequisite invalidation retry policy");
            match ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    ReadinessFailureClassification::Retryable,
                    "prerequisite_invalidated",
                    reason,
                    now_epoch_seconds(),
                    policy,
                )
                .map_err(|error| error.to_string())?
            {
                ReadinessFailureOutcome::RetryScheduled { retry_at } => {
                    Ok(ExecutionOutcome::PrerequisiteInvalidated { retry_at, reason })
                }
                ReadinessFailureOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
                ReadinessFailureOutcome::Permanent
                | ReadinessFailureOutcome::Unsupported
                | ReadinessFailureOutcome::AttemptsExhausted => Ok(ExecutionOutcome::Failed),
            }
        }
    }
}

fn cleanup_unpublished_readiness_output(
    outcome: &Result<ReadinessExecutionOutcome, SourceProcessingFailure>,
) {
    if let Ok(ReadinessExecutionOutcome::Complete(Some(artifact_ref))) = outcome {
        invalidate_persisted_waveform_cache_ref(artifact_ref);
    }
}

// Compatibility-only migration for rows persisted by versions that discarded the execution
// failure type. Live execution always receives `SourceProcessingFailure` from its owner.
fn legacy_unsupported_decode_failure_text(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("failed to decode audio file:")
        || reason.contains("audio decode failed for")
        || reason.contains("audio file contains no complete frames")
        || reason.contains("unsupported codec")
        || reason.contains("no suitable format reader found")
}

fn cancel_claim(
    connection: &mut rusqlite::Connection,
    claim: &ClaimedReadinessWork,
    reason: &str,
    now: i64,
) -> Result<ExecutionOutcome, String> {
    match ReadinessStore::new(connection)
        .cancel(claim, reason, now)
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
                    match ReadinessStore::new(&mut heartbeat_connection).renew_lease(
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
) -> Result<ReadinessExecutionOutcome, SourceProcessingFailure> {
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
                .map_err(SourceProcessingFailure::from)?;
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
                    .map_err(SourceProcessingFailure::from)?;
                if !has_content_hash {
                    let database_root =
                        source.database_root().map_err(|error| error.to_string())?;
                    let db = SourceDatabase::open_for_background_job_with_database_root(
                        &source.root,
                        database_root,
                    )
                    .map_err(source_database_failure)?;
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
                    .map_err(SourceProcessingFailure::from)?
                    .flatten()
                    .filter(|content_hash| !content_hash.is_empty());
                if committed_content_hash.as_deref() == Some(target.content_generation.as_str()) {
                    ReadinessExecutionOutcome::Complete(None)
                } else if committed_content_hash.is_some() {
                    ReadinessExecutionOutcome::PrerequisiteInvalidated(
                        "indexed identity content generation changed",
                    )
                } else {
                    ReadinessExecutionOutcome::Retry(
                        "file is still changing; waiting for a stable content hash",
                    )
                }
            } else {
                ReadinessExecutionOutcome::Retry("indexed identity is not committed yet")
            })
        }
        ReadinessStage::AnalysisFeatures => {
            if target.content_generation.starts_with("pending-") {
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated(
                    "analysis target is waiting for a committed content generation",
                ));
            }
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
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated(
                    "embedding target is waiting for a committed content generation",
                ));
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
                if ReadinessStore::new(connection)
                    .invalidate_artifact(&analysis_target)
                    .map_err(|error| error.to_string())?
                {
                    tracing::warn!(
                        target: "wavecrate::source_processing",
                        source_id = target.source_id,
                        scope_id = target.scope_id,
                        "Invalidated an analysis readiness marker whose payload was missing"
                    );
                }
                return Ok(ReadinessExecutionOutcome::PrerequisiteInvalidated(
                    "analysis prerequisite artifact payload is missing",
                ));
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
            Ok(
                finalize_similarity_artifacts_if_ready(source, &publication_fence, cancel).map(
                    |finalized| {
                        if finalized {
                            ReadinessExecutionOutcome::Complete(None)
                        } else {
                            ReadinessExecutionOutcome::PrerequisiteInvalidated(
                                "similarity prerequisites changed before publication",
                            )
                        }
                    },
                )?,
            )
        }
    }
}

fn readiness_stage_is_unsupported(
    connection: &mut rusqlite::Connection,
    target: &ReadinessTarget,
    stage: &str,
) -> Result<bool, String> {
    let stage = match stage {
        "analysis_features" => ReadinessStage::AnalysisFeatures,
        "embedding_aspects" => ReadinessStage::EmbeddingAspects,
        _ => return Ok(false),
    };
    ReadinessStore::new(connection)
        .stage_is_unsupported(target, stage)
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
                cancel: Arc::new(AtomicBool::new(false)),
                retry_at: 0,
                attempts: 0,
                terminal_offline: false,
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

fn register_source_for_scan_locked(shared: &Shared, source: SampleSource) -> Result<u64, String> {
    let source_id = source.id.as_str().to_string();
    let mut control = shared.control();
    if control.shutdown || shared.cancel.load(Ordering::Acquire) {
        return Err("Source processing supervisor is shutting down".to_string());
    }
    if let Some(current) = control.sources.get(&source_id) {
        if !source_descriptors_match(current, &source) {
            return Err(format!(
                "Source {source_id} is already registered with a different descriptor"
            ));
        }
        let lifecycle_generation = control.source_lifecycle_generations[&source_id];
        if control.quarantined_sources.remove(&source_id) {
            control
                .source_work_cancels
                .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
            control.mark_source_dirty(&source_id, "source_scan_registration_reactivated");
            drop(control);
            shared.budget_wake.notify_all();
            shared.wake.notify_one();
        }
        return Ok(lifecycle_generation);
    }

    control.sources.insert(source_id.clone(), source);
    control
        .source_work_cancels
        .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
    let lifecycle_generation = control.allocate_lifecycle_generation();
    control
        .source_lifecycle_generations
        .insert(source_id.clone(), lifecycle_generation);
    control
        .force_manifest_audit_sources
        .insert(source_id.clone());
    control.mark_source_dirty(&source_id, "source_registered_for_scan");
    drop(control);
    shared.budget_wake.notify_all();
    shared.wake.notify_one();
    Ok(lifecycle_generation)
}

fn resolve_registered_source_for_scan_locked(
    shared: &Shared,
    source: &SampleSource,
) -> Result<u64, String> {
    let source_id = source.id.as_str();
    let control = shared.control();
    if control.shutdown || shared.cancel.load(Ordering::Acquire) {
        return Err("Source processing supervisor is shutting down".to_string());
    }
    let Some(authoritative) = control.sources.get(source_id) else {
        return Err(format!(
            "Source {source_id} is no longer present in the configured source set"
        ));
    };
    if !source_descriptors_match(authoritative, source) {
        return Err(format!(
            "Source {source_id} is registered with a different descriptor"
        ));
    }
    if !control.source_is_active(source_id) {
        return Err(format!("Source {source_id} is not active"));
    }
    control
        .source_lifecycle_generations
        .get(source_id)
        .copied()
        .ok_or_else(|| format!("Source {source_id} has no active lifecycle generation"))
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

fn select_source_for_discovery(
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

fn release_converged_source_owner(
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

fn aggregate_source_stats(
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

fn queue_depths_by_source(candidates: &[RuntimeCandidate]) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    for candidate in candidates {
        *depths
            .entry(candidate.source.id.as_str().to_string())
            .or_default() += 1;
    }
    depths
}

#[cfg(test)]
#[path = "../../test_support/source_processing_liveness/mod.rs"]
mod liveness_tests;

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use wavecrate::sample_sources::{
        SourceId,
        readiness::{
            ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessArtifact, ReadinessDeficit,
            ReadinessEligibility, ReadinessError, ReadinessSnapshot, ReadinessStore,
            ReadinessTargetPublication, ReadinessView, ReadinessWorkStats, SourceAvailability,
        },
    };

    use super::*;

    fn reconcile_readiness(
        connection: &rusqlite::Connection,
        source_id: &str,
        now: i64,
    ) -> Result<ReadinessSnapshot, ReadinessError> {
        ReadinessView::new(connection).reconcile(source_id, now)
    }

    fn reconcile_readiness_with_cancel_and_progress(
        connection: &rusqlite::Connection,
        source_id: &str,
        now: i64,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(),
    ) -> Result<ReadinessSnapshot, ReadinessError> {
        ReadinessView::new(connection)
            .reconcile_with_cancel_and_progress(source_id, now, cancel, progress)
    }

    fn persist_readiness_deficits(
        connection: &mut rusqlite::Connection,
        deficits: &[ReadinessDeficit],
        created_at: i64,
    ) -> Result<usize, ReadinessError> {
        ReadinessStore::new(connection).persist_deficits(deficits, created_at)
    }

    fn publish_readiness_artifact(
        connection: &mut rusqlite::Connection,
        artifact: &ReadinessArtifact,
    ) -> Result<ArtifactPublishOutcome, ReadinessError> {
        ReadinessStore::new(connection).publish_artifact(artifact)
    }

    fn readiness_work_stats(
        connection: &rusqlite::Connection,
        now: i64,
    ) -> Result<ReadinessWorkStats, ReadinessError> {
        ReadinessView::new(connection).work_stats(now)
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_readiness_targets(
        connection: &mut rusqlite::Connection,
        source_id: &str,
        source_generation: i64,
        readiness_revision: i64,
        availability: SourceAvailability,
        targets: &[ReadinessTarget],
        updated_at: i64,
    ) -> Result<(), ReadinessError> {
        ReadinessStore::new(connection).publish_targets(&ReadinessTargetPublication::new(
            source_id,
            source_generation,
            readiness_revision,
            availability,
            "wavecrate-source-readiness-v1",
            targets,
            updated_at,
        ))
    }

    fn claim_readiness_target(
        connection: &mut rusqlite::Connection,
        target: &ReadinessTarget,
        now: i64,
        lease_duration_seconds: i64,
    ) -> Result<Option<ClaimedReadinessWork>, ReadinessError> {
        ReadinessStore::new(connection).claim(target, now, lease_duration_seconds)
    }

    fn complete_readiness_work(
        connection: &mut rusqlite::Connection,
        claim: &ClaimedReadinessWork,
        completed_at: i64,
    ) -> Result<ArtifactPublishOutcome, ReadinessError> {
        ReadinessStore::new(connection).complete(claim, completed_at)
    }

    fn reclassify_known_unsupported_audio_failures(
        connection: &mut rusqlite::Connection,
    ) -> Result<usize, String> {
        ReadinessStore::new(connection)
            .reclassify_known_unsupported_failures(legacy_unsupported_decode_failure_text)
            .map_err(|error| error.to_string())
    }

    fn readiness_stage_is_unsupported(
        connection: &rusqlite::Connection,
        target: &ReadinessTarget,
        stage: &str,
    ) -> Result<bool, String> {
        let stage = match stage {
            "analysis_features" => ReadinessStage::AnalysisFeatures,
            "embedding_aspects" => ReadinessStage::EmbeddingAspects,
            _ => return Ok(false),
        };
        ReadinessView::new(connection)
            .stage_is_unsupported(target, stage)
            .map_err(|error| error.to_string())
    }

    #[test]
    fn playback_active_does_not_block_hash_backlog_and_shutdown_joins() {
        let (_directory, source) = unhashed_source("playing");
        let mut supervisor =
            SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);

        wait_until(Duration::from_secs(10), || source_is_hashed(&source));
        let report = supervisor.shutdown();
        assert_eq!(report["joined"], true);
    }

    #[test]
    fn playback_and_foreground_activity_do_not_publish_pause_feedback() {
        let (_directory, source) = unhashed_source("activity-feedback");
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut supervisor = SourceProcessingSupervisor::start_with_playback_state_and_event_sink(
            vec![source.clone()],
            true,
            Some(Arc::new(sender)),
        );

        supervisor.set_foreground_activity(true);
        wait_until(Duration::from_secs(10), || source_is_hashed(&source));
        let progress = receiver
            .try_iter()
            .filter_map(|event| {
                let SourceProcessingEvent::Progress(progress) = event else {
                    return None;
                };
                Some(progress)
            })
            .collect::<Vec<_>>();
        assert!(
            progress
                .iter()
                .any(|progress| progress.lifecycle.source_id == source.id.as_str()),
            "processing activity must remain visible while playback and foreground loading are active"
        );
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn brief_discovery_reconciliation_does_not_flash_processing_feedback() {
        let source = SampleSource::new_with_id(
            SourceId::from_string("stable-discovery"),
            PathBuf::from("/library/samples"),
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source.clone()], Some(Arc::new(sender)));
        let control = shared.control();
        let lifecycle_generation = control.source_lifecycle_generations[source.id.as_str()];
        drop(control);
        let mut publisher = DiscoveryProgressPublisher {
            shared: &shared,
            source_id: source.id.as_str(),
            lifecycle_generation,
            started_at: Instant::now(),
            last_phase: None,
            last_event_publish_at: None,
            last_log_publish_at: None,
            event_published: false,
        };

        publisher.advance("Reading manifest and readiness targets", 1);
        publisher.advance("Comparing durable readiness", 2);
        assert!(
            receiver.try_recv().is_err(),
            "a brief converged-source check must not flash active processing feedback"
        );
        assert!(!publisher.event_published);

        publisher.started_at = Instant::now() - DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL;
        publisher.advance("Queueing unfinished jobs", 3);
        let SourceProcessingEvent::Progress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("sustained discovery feedback")
        else {
            panic!("unexpected source-processing event");
        };
        assert_eq!(
            progress.activity,
            SourceProcessingActivity::Discovering {
                phase: String::from("Queueing unfinished jobs"),
                completed_steps: 3,
            }
        );
        assert!(
            progress.source_row_active,
            "grace-surviving discovery must identify its active source row"
        );
        assert!(publisher.event_published);
    }

    #[test]
    fn discovery_snapshot_from_previous_readded_epoch_is_not_published() {
        let directory = tempfile::tempdir().expect("discovery source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("readded-discovery-source"),
            directory.path().to_path_buf(),
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Arc::new(Shared::new(vec![source.clone()], Some(Arc::new(sender))));
        let supervisor = SourceProcessingSupervisor {
            shared: Arc::clone(&shared),
            coordinator: None,
            retirement_worker: None,
        };
        let old_generation = shared.control().source_lifecycle_generations[source.id.as_str()];

        supervisor
            .replace_sources(Vec::new())
            .expect("remove old discovery epoch");
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("re-add source with a new discovery epoch");

        let mut publisher = DiscoveryProgressPublisher {
            shared: shared.as_ref(),
            source_id: source.id.as_str(),
            lifecycle_generation: old_generation,
            started_at: Instant::now() - DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL,
            last_phase: None,
            last_event_publish_at: None,
            last_log_publish_at: None,
            event_published: false,
        };
        publisher.advance("Comparing durable readiness", 1);
        assert!(!publisher.event_published);
        assert!(receiver.try_recv().is_err());
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
    fn retirement_cleanup_cannot_block_active_source_convergence() {
        let (_retired_directory, retired) = unhashed_source("retirement-background");
        let (_active_directory, active) = unhashed_source("retirement-active-source");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![retired])
            .expect("configure source that will be retired");
        supervisor
            .replace_sources(vec![active.clone()])
            .expect("replace retired source with active source");

        let shared = Arc::clone(&supervisor.shared);
        let retirement_blocker = shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let coordinator_shared = Arc::clone(&shared);
        supervisor.coordinator = Some(
            thread::Builder::new()
                .name(String::from("wavecrate-source-supervisor-test"))
                .spawn(move || run_coordinator(coordinator_shared))
                .expect("spawn source processing supervisor"),
        );

        wait_until(Duration::from_secs(10), || source_is_hashed(&active));
        drop(retirement_blocker);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn shutdown_cancels_blocked_retirement_cleanup_and_joins_worker() {
        let (_directory, source) = unhashed_source("retirement-shutdown");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        supervisor
            .shared
            .retirement_cleanup_blocked
            .store(true, Ordering::Release);
        supervisor
            .replace_sources(Vec::new())
            .expect("remove source while retirement cleanup is blocked");
        wait_until(Duration::from_secs(5), || {
            supervisor
                .shared
                .retirement_cleanup_started
                .load(Ordering::Acquire)
        });

        let started_at = Instant::now();
        let report = supervisor.shutdown();

        assert_eq!(report["joined"], true);
        assert!(
            started_at.elapsed() < Duration::from_secs(1),
            "shutdown must cancel blocked retirement cleanup before joining"
        );
    }

    #[test]
    fn source_replacement_cancels_blocked_retirement_without_waiting_for_storage() {
        let (_directory, source) = unhashed_source("retirement-replacement");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        supervisor
            .shared
            .retirement_cleanup_blocked
            .store(true, Ordering::Release);
        supervisor
            .replace_sources(Vec::new())
            .expect("remove source while retirement cleanup is blocked");
        wait_until(Duration::from_secs(10), || {
            supervisor
                .shared
                .retirement_cleanup_started
                .load(Ordering::Acquire)
        });

        let started_at = Instant::now();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("re-add source while retirement child is blocked");
        assert!(
            started_at.elapsed() < Duration::from_millis(100),
            "source replacement must cancel retirement instead of waiting for storage"
        );
        wait_until(Duration::from_secs(2), || {
            supervisor.shared.control().pending_retirements.is_empty()
        });
        assert!(
            supervisor
                .shared
                .control()
                .source_is_active(source.id.as_str())
        );
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
    fn busy_source_discovery_is_rescheduled_after_retry_deadline() {
        let (_directory, source) = unhashed_source("busy-discovery-retry");
        let database_path = source.db_path().expect("source database path");
        let lock = rusqlite::Connection::open(&database_path).expect("open lock connection");
        lock.execute_batch("BEGIN EXCLUSIVE")
            .expect("hold source database lock");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);

        wait_until(Duration::from_secs(15), || {
            let telemetry = supervisor.shared.telemetry();
            telemetry.failed > 0
                && telemetry
                    .retry_at_by_source
                    .contains_key(source.id.as_str())
        });
        let retry_at = supervisor
            .shared
            .telemetry()
            .retry_at_by_source
            .get(source.id.as_str())
            .copied();
        assert!(
            retry_at.is_some_and(|deadline| deadline > now_epoch_seconds()),
            "busy discovery must remain observable as a scheduled retry"
        );

        lock.execute_batch("ROLLBACK").expect("release source lock");
        drop(lock);
        wait_until(Duration::from_secs(12), || source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn missing_removed_source_is_parked_offline_without_retry_wakes() {
        let (directory, source) = unhashed_source("retirement-offline");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        supervisor
            .replace_sources(Vec::new())
            .expect("remove source");
        drop(directory);

        process_ready_source_retirements(&supervisor.shared);

        let control = supervisor.shared.control();
        let retirement = control
            .pending_retirements
            .values()
            .next()
            .expect("retain offline retirement fence");
        assert!(retirement.terminal_offline);
        assert_eq!(retirement.retry_at, i64::MAX);
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
    fn bounded_manifest_delta_preserves_unaffected_in_flight_generation() {
        let (_directory, source) = unhashed_source("bounded-delta-generation");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let retained_generation = {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };
        supervisor.request_source_delta(
            source.id.as_str(),
            &CommittedSourceDelta {
                revision: 1,
                changed: vec![wavecrate::sample_sources::scanner::ManifestIdentityDelta {
                    identity: String::from("changed"),
                    relative_path: PathBuf::from("changed.wav"),
                    content_generation: String::from("changed-generation"),
                    source_metadata_changed: false,
                }],
                ..CommittedSourceDelta::default()
            },
            "test_bounded_delta",
        );

        supervisor.wake_source(source.id.as_str(), "filesystem_changed");

        assert!(!retained_generation.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert!(control.dirty_sources.contains(source.id.as_str()));
        assert!(
            control
                .pending_readiness_deltas
                .contains_key(source.id.as_str())
        );
        assert!(Arc::ptr_eq(
            &retained_generation,
            &control.source_work_cancels[source.id.as_str()]
        ));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn non_mutating_source_requests_preserve_in_flight_generation() {
        let (_directory, source) = unhashed_source("non-mutating-source-request");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let retained_generation = {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };

        supervisor.request_source_processing(source.id.as_str(), "source_scan_finished");

        assert!(!retained_generation.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert!(control.dirty_sources.contains(source.id.as_str()));
        assert!(Arc::ptr_eq(
            &retained_generation,
            &control.source_work_cancels[source.id.as_str()]
        ));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn explicit_reanalysis_cancels_current_work_without_implicit_priority() {
        let (_directory, source) = unhashed_source("explicit-reanalysis-request");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        let retained_generation = {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            Arc::clone(&control.source_work_cancels[source.id.as_str()])
        };
        let scan = supervisor
            .budget_handle()
            .acquire_scan(source.id.as_str())
            .expect("admit source scan");
        let scan_cancel = scan.cancel_token();

        supervisor.request_source_reanalysis(source.id.as_str(), "user_process_source");

        assert!(retained_generation.load(Ordering::Acquire));
        assert!(scan_cancel.load(Ordering::Acquire));
        let control = supervisor.shared.control();
        assert!(control.dirty_sources.contains(source.id.as_str()));
        assert!(
            control
                .force_reanalysis_sources
                .contains(source.id.as_str())
        );
        assert_eq!(control.priority.selected_source, None);
        assert!(
            !control.source_work_cancels[source.id.as_str()].load(Ordering::Acquire),
            "the replacement generation must be available for the reanalysis run"
        );
        drop(control);
        drop(scan);
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
    fn background_scan_registration_waits_for_source_replacement_fence() {
        let (_directory, source) = unhashed_source("scan-registration-waiting");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure authoritative source");
        let replacement = supervisor
            .shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let budget = supervisor.budget_handle();
        let source_for_worker = source.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = budget.register_source_for_scan_waiting(source_for_worker);
            sender.send(result).expect("publish registration result");
        });

        assert!(
            receiver.recv_timeout(Duration::from_millis(25)).is_err(),
            "background admission should wait while source replacement owns the fence"
        );
        drop(replacement);
        let generation = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("registration should resume after replacement")
            .expect("register matching source");
        worker.join().expect("join registration worker");

        assert_eq!(
            supervisor.lifecycle_generations()[source.id.as_str()],
            generation
        );
        let permit = supervisor
            .budget_handle()
            .acquire_scan_for_generation(source.id.as_str(), generation)
            .expect("deferred registration must admit the external scan");
        drop(permit);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn background_scan_registration_cannot_readd_source_removed_behind_fence() {
        let (_directory, source) = unhashed_source("scan-registration-removed");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        let replacement = supervisor
            .shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let budget = supervisor.budget_handle();
        let source_for_worker = source.clone();
        let worker =
            std::thread::spawn(move || budget.register_source_for_scan_waiting(source_for_worker));

        thread::sleep(Duration::from_millis(25));
        drop(replacement);
        let error = worker
            .join()
            .expect("join deferred scan registration")
            .expect_err("removed source must not be registered by a stale scan");

        assert!(error.contains("no longer present"));
        assert!(
            !supervisor
                .lifecycle_generations()
                .contains_key(source.id.as_str()),
            "stale scan admission must not resurrect a removed source"
        );
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn watcher_ready_rearms_authoritative_manifest_audits() {
        let (_directory, source) = unhashed_source("watcher-ready-audit");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        {
            let mut control = supervisor.shared.control();
            control.force_manifest_audit_sources.clear();
            control.dirty_sources.clear();
        }

        supervisor.request_manifest_audits("source_watcher_ready");

        let control = supervisor.shared.control();
        assert!(
            control
                .force_manifest_audit_sources
                .contains(source.id.as_str())
        );
        assert!(control.dirty_sources.contains(source.id.as_str()));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn watcher_ready_request_survives_older_in_flight_audit_completion() {
        let (_directory, source) = unhashed_source("watcher-ready-in-flight-audit");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        {
            let mut control = supervisor.shared.control();
            // Model the coordinator having already captured the original
            // startup audit request and started its candidate.
            control.dirty_sources.clear();
            assert!(
                control
                    .force_manifest_audit_sources
                    .contains(source.id.as_str())
            );
        }

        supervisor.request_manifest_audits("source_watcher_ready");
        clear_satisfied_manifest_audit_request(&supervisor.shared, source.id.as_str());

        {
            let mut control = supervisor.shared.control();
            assert!(control.dirty_sources.contains(source.id.as_str()));
            assert!(
                control
                    .force_manifest_audit_sources
                    .contains(source.id.as_str()),
                "the older audit must not erase the watcher-ready closing audit"
            );
            // Once the coordinator captures that newer dirty request, its own
            // successful audit may satisfy and clear the force flag.
            control.dirty_sources.remove(source.id.as_str());
        }
        clear_satisfied_manifest_audit_request(&supervisor.shared, source.id.as_str());
        assert!(
            !supervisor
                .shared
                .control()
                .force_manifest_audit_sources
                .contains(source.id.as_str())
        );
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn external_admission_rejects_generation_captured_before_descriptor_replacement() {
        let old_directory = tempfile::tempdir().expect("old source root");
        let replacement_directory = tempfile::tempdir().expect("replacement source root");
        let source_id = SourceId::from_string("replaced-external-admission");
        let old_source =
            SampleSource::new_with_id(source_id.clone(), old_directory.path().to_path_buf());
        let replacement =
            SampleSource::new_with_id(source_id, replacement_directory.path().to_path_buf());
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![old_source.clone()])
            .expect("configure old descriptor");
        let handle = supervisor.budget_handle();
        let old_generation = handle
            .lifecycle_generation(old_source.id.as_str())
            .expect("capture queued request generation");

        supervisor
            .replace_sources(vec![replacement])
            .expect("replace source descriptor before admission");

        assert!(
            handle
                .acquire_scan_for_generation(old_source.id.as_str(), old_generation)
                .is_none(),
            "a queued request must not adopt the replacement descriptor generation"
        );
        assert_ne!(
            handle
                .lifecycle_generation(old_source.id.as_str())
                .expect("replacement generation"),
            old_generation
        );
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
            SourceId::from_string(format!("priority-cache-{}", uuid::Uuid::new_v4())),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create priority source database");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
        // The empty-source fixture converges through startup, manifest-audit, and readiness
        // handoffs. Do not capture the baseline at a transient queue-empty boundary between them.
        wait_until(Duration::from_secs(5), || {
            let telemetry = supervisor.shared.telemetry();
            let completed = telemetry.completed;
            let queue_depth = telemetry.queue_depth;
            let settled_wake_generation = telemetry.settled_wake_generation;
            drop(telemetry);
            let control = supervisor.shared.control();
            completed >= 2
                && queue_depth == 0
                && settled_wake_generation == control.wake_generation
                && control.dirty_sources.is_empty()
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
            SourceId::from_string(format!("resume-cache-{}", uuid::Uuid::new_v4())),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create resume source database");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source]);
        // The empty-source fixture converges through startup, manifest-audit, and readiness
        // handoffs. Do not capture the baseline at a transient queue-empty boundary between them.
        wait_until(Duration::from_secs(5), || {
            let telemetry = supervisor.shared.telemetry();
            let completed = telemetry.completed;
            let queue_depth = telemetry.queue_depth;
            let settled_wake_generation = telemetry.settled_wake_generation;
            drop(telemetry);
            let control = supervisor.shared.control();
            completed >= 2
                && queue_depth == 0
                && settled_wake_generation == control.wake_generation
                && control.dirty_sources.is_empty()
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
        for retirement in supervisor.shared.control().pending_retirements.values_mut() {
            retirement.retry_at = now_epoch_seconds().saturating_add(60);
        }
        let waiting_handle = supervisor.budget_handle();
        let waiting_source_id = source.id.as_str().to_string();
        let (waiting_sender, waiting_receiver) = std::sync::mpsc::channel();
        let waiting = thread::spawn(move || {
            let permit = waiting_handle.acquire_scan(&waiting_source_id);
            waiting_sender
                .send(permit)
                .expect("report re-added source admission");
        });
        assert!(
            waiting_receiver
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "same-storage admission must wait while the retired epoch is still active"
        );
        process_ready_source_retirements(&supervisor.shared);
        assert_eq!(supervisor.shared.control().pending_retirements.len(), 1);

        drop(old_work);
        process_ready_source_retirements(&supervisor.shared);
        assert!(supervisor.shared.control().pending_retirements.is_empty());
        let permit = waiting_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("re-added source admission should resume after the old epoch drains")
            .expect("re-added source receives its current-generation permit");
        assert_eq!(
            permit.lifecycle_generation(),
            supervisor.lifecycle_generations()[source.id.as_str()]
        );
        drop(permit);
        waiting.join().expect("join re-added source admission");
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
    fn converged_periodic_safety_probes_do_not_rematerialize_targets() {
        let (_directory, source) = unhashed_source("revision-gated-safety-probe");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
            .expect("publish initial readiness");
        let durable_before = discovery_durable_counts(&connection);
        let source_before = ReadinessStore::new(&mut connection)
            .source_state(source.id.as_str())
            .expect("read source state")
            .expect("source state exists");
        drop(connection);
        let cancel = AtomicBool::new(false);

        for interval in 0..10 {
            let Cancellable::Completed((candidates, stats)) =
                discover_source_candidates_with_progress(
                    &source,
                    101 + interval,
                    false,
                    false,
                    None,
                    true,
                    &cancel,
                    &mut |_, _| panic!("cheap safety probe must not materialize target work"),
                )
                .expect("run revision-gated safety probe")
            else {
                panic!("safety probe unexpectedly cancelled");
            };
            assert!(candidates.is_empty());
            assert!(stats.cheap_noop_sweep);
        }

        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen source database");
        assert_eq!(discovery_durable_counts(&connection), durable_before);
        assert_eq!(
            ReadinessStore::new(&mut connection)
                .source_state(source.id.as_str())
                .expect("read final source state")
                .expect("source state exists"),
            source_before
        );
    }

    #[test]
    fn safety_probe_recovers_manifest_commit_without_delta_publication() {
        let (_directory, source) = unhashed_source("revision-gap-recovery");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
            .expect("publish initial readiness");
        let previous_generation = ReadinessStore::new(&mut connection)
            .source_state(source.id.as_str())
            .expect("read source state")
            .expect("source state exists")
            .source_generation;
        connection
            .execute(
                "UPDATE wav_files SET content_hash = 'changed-after-crash'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("commit manifest content change");
        connection
            .execute(
                "INSERT INTO metadata (key, value) VALUES (?1, '1')
                 ON CONFLICT(key) DO UPDATE
                 SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)",
                [META_WAV_PATHS_REVISION],
            )
            .expect("advance manifest generation");

        let Cancellable::Completed((_candidates, stats)) =
            discover_source_candidates_with_connection_and_progress(
                &source,
                &mut connection,
                101,
                false,
                false,
                None,
                true,
                &AtomicBool::new(false),
                &mut |_, _| {},
            )
            .expect("recover readiness publication")
        else {
            panic!("recovery unexpectedly cancelled");
        };
        assert!(!stats.cheap_noop_sweep);
        let recovered = ReadinessStore::new(&mut connection)
            .source_state(source.id.as_str())
            .expect("read recovered source state")
            .expect("source state exists");
        assert_eq!(
            recovered.source_generation,
            previous_generation.saturating_add(1)
        );
    }

    #[test]
    fn committed_one_file_delta_updates_only_that_identity_targets() {
        let (_directory, source) = unhashed_source("one-file-readiness-delta");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
            .expect("publish initial readiness");
        let identity: String = connection
            .query_row(
                "SELECT file_identity FROM wav_files WHERE path = 'pending.wav'",
                [],
                |row| row.get(0),
            )
            .expect("read file identity");
        let state_before = ReadinessStore::new(&mut connection)
            .source_state(source.id.as_str())
            .expect("read source state")
            .expect("source state exists");
        connection
            .execute(
                "UPDATE wav_files
                 SET content_hash = 'one-file-new-hash',
                     file_size = file_size + 1,
                     modified_ns = modified_ns + 1
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("commit one-file manifest change");
        connection
            .execute(
                "INSERT INTO metadata (key, value) VALUES (?1, '1')
                 ON CONFLICT(key) DO UPDATE
                 SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)",
                [META_WAV_PATHS_REVISION],
            )
            .expect("advance manifest generation");
        let delta = PendingReadinessDelta {
            scope_ids: [identity.clone()].into_iter().collect(),
        };

        let Cancellable::Completed((_candidates, stats)) =
            discover_source_candidates_with_connection_and_progress(
                &source,
                &mut connection,
                101,
                false,
                false,
                Some(&delta),
                false,
                &AtomicBool::new(false),
                &mut |_, _| {},
            )
            .expect("reconcile committed delta")
        else {
            panic!("delta reconciliation unexpectedly cancelled");
        };
        assert!(stats.delta_reconciled);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1 AND scope_kind = 'file'",
                    [source.id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count file targets"),
            3
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1
                       AND scope_id = ?2
                       AND content_generation = 'one-file-new-hash'",
                    params![source.id.as_str(), identity],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count changed identity targets"),
            3
        );
        let state_after = ReadinessStore::new(&mut connection)
            .source_state(source.id.as_str())
            .expect("read source state")
            .expect("source state exists");
        assert_eq!(
            state_after.source_generation,
            state_before.source_generation.saturating_add(1)
        );
        assert_eq!(
            state_after.readiness_revision,
            state_before.readiness_revision.saturating_add(1)
        );
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
            discover_source_candidates(&source, 100, false, &cancel)
                .expect("discover large source")
        else {
            panic!("large source discovery unexpectedly cancelled");
        };
        let elapsed = started_at.elapsed();

        assert_eq!(candidates.len(), FILE_COUNT * 3);
        assert_eq!(stats.readiness_queue_depth, FILE_COUNT * 3);
        assert_eq!(
            stats.prerequisites_blocked,
            FILE_COUNT * 3,
            "the source-wide similarity layout must remain parked until file embeddings converge"
        );
        eprintln!(
            "large_source_discovery file_count={FILE_COUNT} candidate_count={} elapsed_ms={:.3}",
            candidates.len(),
            elapsed.as_secs_f64() * 1_000.0,
        );
    }

    #[test]
    fn discovery_reports_monotonic_work_backed_progress() {
        const FILE_COUNT: usize = 8;
        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("discovery-progress"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create progress source database");
        let database_root = source.database_root().expect("progress database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open progress database");
        let transaction = connection.transaction().expect("start progress seed");
        for index in 0..FILE_COUNT {
            transaction
                .execute(
                    "INSERT INTO wav_files (
                        path, file_size, modified_ns, file_identity, content_hash, missing,
                        extension
                     ) VALUES (?1, 1024, 1, ?2, ?3, 0, 'wav')",
                    params![
                        format!("progress/sample-{index:02}.wav"),
                        format!("progress-identity-{index:02}"),
                        format!("progress-content-{index:02}"),
                    ],
                )
                .expect("insert progress row");
        }
        transaction.commit().expect("commit progress seed");
        drop(connection);

        let mut updates = Vec::new();
        let Cancellable::Completed(_) = discover_source_candidates_with_progress(
            &source,
            100,
            false,
            false,
            None,
            false,
            &AtomicBool::new(false),
            &mut |phase, work_units| updates.push((phase, work_units)),
        )
        .expect("discover source with progress") else {
            panic!("progress discovery unexpectedly cancelled");
        };

        assert!(updates.len() > FILE_COUNT);
        assert!(
            updates.windows(2).all(|pair| pair[0].1 <= pair[1].1),
            "discovery work units must never move backward"
        );
        assert!(updates.last().unwrap().1 > updates.first().unwrap().1);
        assert!(
            updates
                .iter()
                .any(|(phase, _)| *phase == "Comparing durable readiness")
        );
        assert!(
            updates
                .iter()
                .any(|(phase, _)| *phase == "Queueing unfinished jobs")
        );
    }

    #[test]
    fn discovery_progress_publisher_exposes_advancing_counter() {
        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("progress-publisher"),
            directory.path().to_path_buf(),
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source], Some(Arc::new(sender)));
        let lifecycle_generation =
            shared.control().source_lifecycle_generations["progress-publisher"];
        let mut publisher = DiscoveryProgressPublisher {
            shared: &shared,
            source_id: "progress-publisher",
            lifecycle_generation,
            started_at: Instant::now() - DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL,
            last_phase: None,
            last_event_publish_at: None,
            last_log_publish_at: None,
            event_published: false,
        };

        publisher.advance("Comparing durable readiness", 128);
        publisher.last_event_publish_at =
            Some(Instant::now() - DISCOVERY_PROGRESS_REFRESH_INTERVAL);
        publisher.advance("Comparing durable readiness", 256);

        let updates = receiver
            .try_iter()
            .filter_map(|event| match event {
                SourceProcessingEvent::Progress(progress) => Some(progress),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].lifecycle.source_id, "progress-publisher");
        assert_eq!(updates[0].lifecycle.generation, lifecycle_generation);
        assert_eq!(
            updates[0].activity,
            SourceProcessingActivity::Discovering {
                phase: String::from("Comparing durable readiness"),
                completed_steps: 128,
            }
        );
        assert_eq!(
            updates[1].activity,
            SourceProcessingActivity::Discovering {
                phase: String::from("Comparing durable readiness"),
                completed_steps: 256,
            }
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
            false,
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
            i64::try_from(FILE_COUNT * 3 + 1).expect("target count fits i64")
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
    fn foreground_scan_admission_waits_without_cancelling_background_work() {
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
            shared.external_scans().admissions.len() == 1
        });
        assert!(
            !background_cancel.load(Ordering::Acquire),
            "external scan admission must let active source work finish"
        );
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
    fn foreground_activity_does_not_cancel_in_flight_work_or_external_scans() {
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

        assert!(!source_generation.load(Ordering::Acquire));
        assert!(!scan_generation.load(Ordering::Acquire));
        drop(scan_permit);

        supervisor.set_foreground_activity(false);

        let control = supervisor.shared.control();
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
            discover_source_candidates_with_connection(
                &source,
                &mut connection,
                100,
                false,
                &cancel,
            )
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
            discover_source_candidates(&source, MANIFEST_AUDIT_INTERVAL_SECONDS, false, &cancel)
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
        let Cancellable::Completed((not_due, _)) = discover_source_candidates(
            &source,
            MANIFEST_AUDIT_INTERVAL_SECONDS * 2 - 1,
            false,
            &cancel,
        )
        .expect("discover recent manifest audit") else {
            panic!("manifest audit discovery unexpectedly cancelled");
        };
        assert!(
            not_due
                .iter()
                .all(|candidate| !matches!(candidate.task, RuntimeTask::ManifestAudit))
        );

        let Cancellable::Completed((forced, _)) = discover_source_candidates(
            &source,
            MANIFEST_AUDIT_INTERVAL_SECONDS * 2 - 1,
            true,
            &cancel,
        )
        .expect("discover forced startup manifest audit") else {
            panic!("forced manifest audit discovery unexpectedly cancelled");
        };
        assert!(
            forced
                .iter()
                .any(|candidate| matches!(candidate.task, RuntimeTask::ManifestAudit))
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
            discover_source_candidates(&source, 100, false, &cancel)
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
            discover_source_candidates(&source, 100, false, &cancel)
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
            discover_source_candidates(&source, 100, false, &cancel)
                .expect("discover unavailable source")
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
            execute_candidate(&candidate, 0, &AtomicBool::new(false), &mut |_| false,)
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
        let shared = Shared::new(vec![source], Some(Arc::new(sender)));
        let lifecycle_generation = shared.control().source_lifecycle_generations["progress-source"];

        publish_source_processing_progress(
            &shared,
            &candidate,
            lifecycle_generation,
            SourceDiscoveryStats {
                progress_completed: 313,
                progress_total: 9_985,
                ..SourceDiscoveryStats::default()
            },
        );

        let event = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("progress event");
        let SourceProcessingEvent::Progress(progress) = event else {
            panic!("unexpected source-processing event: {event:?}");
        };
        assert_eq!(progress.lifecycle.source_id, "progress-source");
        assert_eq!(progress.completed, 313);
        assert_eq!(progress.total, 9_985);
        assert_eq!(
            progress.activity,
            SourceProcessingActivity::Readiness {
                stage: ReadinessStage::EmbeddingAspects,
                relative_path: Some(String::from("drums/kick.wav")),
            }
        );
    }

    #[test]
    fn prerequisite_wait_feedback_preserves_determinate_progress_without_claiming_activity() {
        let directory = tempfile::tempdir().expect("waiting source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("waiting-source"),
            directory.path().to_path_buf(),
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Shared::new(vec![source.clone()], Some(Arc::new(sender)));
        let lifecycle_generation = shared.control().source_lifecycle_generations["waiting-source"];
        let lifecycle_generations =
            BTreeMap::from([(String::from("waiting-source"), lifecycle_generation)]);
        let mut source_stats = BTreeMap::from([(
            String::from("waiting-source"),
            SourceDiscoveryStats {
                prerequisites_blocked: 1,
                earliest_retry_at: Some(now_epoch_seconds().saturating_add(60)),
                progress_completed: 72,
                progress_total: 77,
                ..SourceDiscoveryStats::default()
            },
        )]);

        assert!(publish_source_processing_prerequisite_wait(
            &shared,
            &lifecycle_generations,
            &source_stats,
        ));

        let SourceProcessingEvent::Progress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("blocked prerequisite message")
        else {
            panic!("unexpected source-processing event");
        };
        assert_eq!(progress.lifecycle.source_id, "waiting-source");
        assert_eq!(progress.lifecycle.generation, lifecycle_generation);
        assert_eq!(progress.completed, 72);
        assert_eq!(progress.total, 77);
        assert_eq!(
            progress.activity,
            SourceProcessingActivity::WaitingForPrerequisites { retry_at: None }
        );

        source_stats
            .get_mut("waiting-source")
            .expect("waiting source stats")
            .prerequisite_retry_at = Some(now_epoch_seconds().saturating_add(60));
        assert!(publish_source_processing_prerequisite_wait(
            &shared,
            &lifecycle_generations,
            &source_stats,
        ));
        let SourceProcessingEvent::Progress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("retrying prerequisite message")
        else {
            panic!("unexpected source-processing event");
        };
        assert!(matches!(
            progress.activity,
            SourceProcessingActivity::WaitingForPrerequisites { retry_at: Some(_) }
        ));
    }

    #[test]
    fn similarity_blocker_state_ignores_unrelated_retries_and_non_file_targets() {
        let source_id = "dependency-specific-retry";
        let layout = ReadinessTarget::source(
            source_id,
            ReadinessStage::SimilarityLayout,
            "layout-v1",
            1,
            "membership-v1",
        );
        let file_target = |stage| {
            ReadinessTarget::file(
                source_id,
                "identity-1",
                "kick.wav",
                stage,
                "v1",
                1,
                "content-1",
            )
        };
        let mut snapshot = ReadinessSnapshot {
            source_id: source_id.to_string(),
            source_generation: 1,
            readiness_revision: 1,
            availability: SourceAvailability::Active,
            entries: vec![
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: layout.clone(),
                    classification: ReadinessClassification::Pending,
                },
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: file_target(ReadinessStage::IndexedIdentity),
                    classification: ReadinessClassification::Current,
                },
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: file_target(ReadinessStage::AnalysisFeatures),
                    classification: ReadinessClassification::Current,
                },
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: file_target(ReadinessStage::EmbeddingAspects),
                    classification: ReadinessClassification::PermanentFailure {
                        reason: String::from("embedding failed permanently"),
                    },
                },
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: {
                        let mut target = file_target(ReadinessStage::AnalysisFeatures);
                        target.source_id = String::from("unrelated-source");
                        target
                    },
                    classification: ReadinessClassification::RetryableFailure {
                        retry_at: 200,
                        reason: String::from("unrelated source retry"),
                    },
                },
                wavecrate::sample_sources::readiness::ReadinessEntry {
                    target: ReadinessTarget::source(
                        source_id,
                        ReadinessStage::AnalysisFeatures,
                        "malformed-source-analysis-v1",
                        1,
                        "malformed-source-analysis-generation",
                    ),
                    classification: ReadinessClassification::Pending,
                },
            ],
            deficits: Vec::new(),
            stage_counts: BTreeMap::new(),
            activity: wavecrate::sample_sources::readiness::ReadinessActivity::Idle,
        };

        assert_eq!(similarity_prerequisite_blocker_stats(&snapshot), (1, None));

        snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.target.stage == ReadinessStage::EmbeddingAspects)
            .expect("embedding blocker")
            .classification = ReadinessClassification::RetryableFailure {
            retry_at: 300,
            reason: String::from("embedding retry"),
        };
        assert_eq!(
            similarity_prerequisite_blocker_stats(&snapshot),
            (1, Some(300))
        );

        snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.target.stage == ReadinessStage::EmbeddingAspects)
            .expect("embedding blocker")
            .classification = ReadinessClassification::Current;
        assert!(snapshot.prerequisites_are_current(&layout));
        assert_eq!(similarity_prerequisite_blocker_stats(&snapshot), (0, None));
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
        let shared = Shared::new(vec![source], Some(Arc::new(sender)));
        let lifecycle_generation = shared.control().source_lifecycle_generations["boundary-source"];

        publish_source_processing_progress(
            &shared,
            &candidate,
            lifecycle_generation,
            SourceDiscoveryStats {
                progress_completed: 25_000,
                progress_total: 25_000,
                ..SourceDiscoveryStats::default()
            },
        );

        let SourceProcessingEvent::Progress(progress) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("progress event")
        else {
            panic!("unexpected source-processing event");
        };
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.total, 0);
        assert!(matches!(
            progress.activity,
            SourceProcessingActivity::Readiness {
                stage: ReadinessStage::AnalysisFeatures,
                ..
            }
        ));
    }

    #[test]
    fn late_progress_is_rejected_across_remove_and_readd() {
        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("readded-progress-source"),
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
        let shared = Arc::new(Shared::new(vec![source.clone()], Some(Arc::new(sender))));
        let supervisor = SourceProcessingSupervisor {
            shared: Arc::clone(&shared),
            coordinator: None,
            retirement_worker: None,
        };
        let executing_generation = supervisor.lifecycle_generations()["readded-progress-source"];

        supervisor
            .replace_sources(Vec::new())
            .expect("remove source while work is executing");
        supervisor
            .replace_sources(vec![source])
            .expect("re-add source before late progress is published");
        let readded_generation = supervisor.lifecycle_generations()["readded-progress-source"];
        assert_ne!(executing_generation, readded_generation);

        publish_source_processing_progress(
            shared.as_ref(),
            &candidate,
            executing_generation,
            SourceDiscoveryStats {
                progress_completed: 1,
                progress_total: 2,
                ..SourceDiscoveryStats::default()
            },
        );

        assert!(
            receiver.try_recv().is_err(),
            "an event from a retired lifecycle must be fenced before reaching the sink"
        );
    }

    #[test]
    fn lifecycle_fence_remains_held_until_event_delivery_finishes() {
        #[derive(Default)]
        struct BlockingSink {
            state: Mutex<(bool, bool, Vec<SourceProcessingEvent>)>,
            wake: Condvar,
        }

        impl SourceProcessingEventSink for BlockingSink {
            fn try_publish(&self, event: SourceProcessingEvent) -> bool {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                state.0 = true;
                self.wake.notify_all();
                while !state.1 {
                    state = self
                        .wake
                        .wait(state)
                        .unwrap_or_else(|poison| poison.into_inner());
                }
                state.2.push(event);
                true
            }
        }

        let directory = tempfile::tempdir().expect("progress source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("atomic-progress-source"),
            directory.path().to_path_buf(),
        );
        let sink = Arc::new(BlockingSink::default());
        let shared = Arc::new(Shared::new(
            vec![source.clone()],
            Some(Arc::clone(&sink) as Arc<dyn SourceProcessingEventSink>),
        ));
        let lifecycle_generation =
            shared.control().source_lifecycle_generations[source.id.as_str()];
        let publisher_shared = Arc::clone(&shared);
        let publisher = thread::spawn(move || {
            publisher_shared.publish_event(SourceProcessingEvent::Progress(
                SourceProcessingProgressEvent {
                    lifecycle: SourceProcessingLifecycle::new(
                        "atomic-progress-source",
                        lifecycle_generation,
                    ),
                    source_row_active: true,
                    completed: 1,
                    total: 2,
                    activity: SourceProcessingActivity::Readiness {
                        stage: ReadinessStage::AnalysisFeatures,
                        relative_path: Some(String::from("drums/kick.wav")),
                    },
                },
            ))
        });

        let mut sink_state = sink
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        while !sink_state.0 {
            sink_state = sink
                .wake
                .wait(sink_state)
                .unwrap_or_else(|poison| poison.into_inner());
        }
        drop(sink_state);

        let replacement_supervisor = SourceProcessingSupervisor {
            shared: Arc::clone(&shared),
            coordinator: None,
            retirement_worker: None,
        };
        let (replacement_started, replacement_started_rx) = std::sync::mpsc::channel();
        let (replacement_finished, replacement_finished_rx) = std::sync::mpsc::channel();
        let replacement = thread::spawn(move || {
            replacement_started.send(()).expect("replacement start");
            replacement_supervisor
                .replace_sources(Vec::new())
                .expect("remove source");
            replacement_supervisor
                .replace_sources(vec![source])
                .expect("re-add source");
            replacement_finished.send(()).expect("replacement finish");
            replacement_supervisor
        });
        replacement_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("replacement thread started");
        assert!(
            replacement_finished_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "source replacement must wait until admitted event delivery finishes"
        );

        let mut sink_state = sink
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        sink_state.1 = true;
        sink.wake.notify_all();
        drop(sink_state);

        assert!(publisher.join().expect("publisher joined"));
        replacement_finished_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("replacement finished after event delivery");
        let replacement_supervisor = replacement.join().expect("replacement joined");
        assert_ne!(
            replacement_supervisor.lifecycle_generations()["atomic-progress-source"],
            lifecycle_generation
        );
        assert_eq!(
            sink.state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .2
                .len(),
            1
        );
    }

    #[test]
    fn readiness_progress_counts_remain_scoped_to_the_reported_source() {
        let mut source_stats = BTreeMap::from([
            (
                String::from("first"),
                SourceDiscoveryStats {
                    readiness_queue_depth: 1,
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
        assert_eq!(progress.readiness_queue_depth, 0);
        assert_eq!(source_stats["second"].progress_completed, 24_000);
        assert_eq!(source_stats["second"].progress_total, 25_000);
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
            execute_candidate(&candidate, 0, &AtomicBool::new(false), &mut |event| sender
                .send(event)
                .is_ok(),)
            .expect("execute manifest audit"),
            ExecutionOutcome::CompletedAwaitingForegroundRefresh
        );
        let events = receiver.try_iter().collect::<Vec<_>>();
        let progress = events
            .iter()
            .find_map(|event| match event {
                SourceProcessingEvent::Progress(progress) => Some(progress),
                _ => None,
            })
            .expect("audit should publish checked-file progress");
        let (lifecycle, committed_delta) = events
            .iter()
            .find_map(|event| match event {
                SourceProcessingEvent::ManifestAuditCommitted {
                    lifecycle,
                    committed_delta,
                } => Some((lifecycle, committed_delta)),
                _ => None,
            })
            .expect("audit should publish a browser projection wake");

        assert_eq!(lifecycle.source_id, "audit-browser-wake");
        assert_eq!(lifecycle.generation, 0);
        assert_eq!(progress.lifecycle.source_id, "audit-browser-wake");
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.total, 1);
        assert!(
            !progress.source_row_active,
            "manifest maintenance remains visible without claiming the active source pulse"
        );
        assert_eq!(
            progress.activity,
            SourceProcessingActivity::ManifestAudit {
                checked: Some(1),
                relative_path: Some(PathBuf::from("missed.wav")),
            }
        );
        assert_eq!(committed_delta.created.len(), 1);
        assert_eq!(
            committed_delta.created[0].relative_path,
            Path::new("missed.wav")
        );
    }

    #[test]
    fn delivered_manifest_handoff_survives_post_commit_cancellation() {
        assert_eq!(
            manifest_audit_execution_outcome(true, false, true),
            ExecutionOutcome::CompletedAwaitingForegroundRefresh
        );
        assert_eq!(
            manifest_audit_execution_outcome(true, true, true),
            ExecutionOutcome::FailedAwaitingForegroundRefresh
        );
        assert_eq!(
            manifest_audit_execution_outcome(false, false, true),
            ExecutionOutcome::Cancelled
        );
    }

    #[test]
    fn manifest_projection_handoff_defers_discovery_until_external_scan_releases() {
        let directory = tempfile::tempdir().expect("manifest handoff source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("audit-foreground-handoff"),
            directory.path().to_path_buf(),
        );
        source.open_db().expect("create source database");
        std::fs::write(directory.path().join("missed.wav"), [7_u8; 32])
            .expect("write missed watcher file");
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut supervisor = SourceProcessingSupervisor::start_with_playback_state_and_event_sink(
            vec![source.clone()],
            false,
            Some(Arc::new(sender)),
        );

        let deadline = Instant::now() + Duration::from_secs(10);
        let lifecycle_generation = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let event = receiver
                .recv_timeout(remaining)
                .expect("manifest audit should publish its committed delta");
            if let SourceProcessingEvent::ManifestAuditCommitted {
                lifecycle,
                committed_delta,
            } = event
                && lifecycle.source_id == source.id.as_str()
                && !committed_delta.created.is_empty()
            {
                break lifecycle.generation;
            }
        };
        let permit = supervisor
            .budget_handle()
            .acquire_scan_for_generation(source.id.as_str(), lifecycle_generation)
            .expect("foreground refresh should acquire the source scan lane");
        let discoveries_before = supervisor.shared.telemetry().source_discoveries;
        supervisor.wake_source(source.id.as_str(), "change_during_manifest_refresh");

        thread::sleep(Duration::from_millis(250));
        assert_eq!(
            supervisor.shared.telemetry().source_discoveries,
            discoveries_before,
            "coordinator must not rediscover while foreground refresh owns reconciliation"
        );

        drop(permit);
        thread::sleep(Duration::from_millis(100));
        assert_eq!(
            supervisor.shared.telemetry().source_discoveries,
            discoveries_before,
            "scan permit release alone must not bypass SourceScanFinished reconciliation"
        );
        supervisor.finish_foreground_source_refresh(source.id.as_str(), "source_scan_finished");
        wait_until(Duration::from_secs(10), || {
            supervisor.shared.telemetry().source_discoveries > discoveries_before
        });
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn unavailable_foreground_refresh_releases_handoff_for_offline_discovery() {
        let (directory, source) = unhashed_source("audit-refresh-unavailable");
        let mut supervisor = SourceProcessingSupervisor::dormant();
        supervisor
            .replace_sources(vec![source.clone()])
            .expect("configure source");
        {
            let mut control = supervisor.shared.control();
            control.dirty_sources.clear();
            control
                .awaiting_foreground_refresh_sources
                .insert(source.id.as_str().to_string());
        }
        drop(directory);

        supervisor.finish_foreground_source_refresh(
            source.id.as_str(),
            "source_refresh_root_unavailable",
        );

        let control = supervisor.shared.control();
        assert!(
            !control
                .awaiting_foreground_refresh_sources
                .contains(source.id.as_str())
        );
        assert!(control.dirty_sources.contains(source.id.as_str()));
        drop(control);
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn sustained_manifest_audit_activates_source_row_after_grace_period() {
        assert!(
            !manifest_audit_source_row_active(Instant::now()),
            "brief manifest maintenance must not flash the source row"
        );
        assert!(
            manifest_audit_source_row_active(
                Instant::now() - DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL
            ),
            "a sustained manifest audit must identify its active source row"
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
                          AND artifact.artifact_version = target.required_version
                          AND artifact.content_generation = target.content_generation
                          AND target.stage != 'playback_summary'
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
        let database_root = source.database_root().expect("database root");
        let connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open converged readiness database");
        let playback_rows = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary')",
                [source.id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .expect("count playback readiness rows");
        assert_eq!(
            playback_rows, 0,
            "source convergence must not create persistent playback work"
        );
    }

    #[test]
    fn legacy_playback_readiness_is_retired_without_requeueing_source_work() {
        let (_directory, source) = ready_analysis_source("legacy-playback-retirement");
        let database_root = source.database_root().expect("database root");
        let (cache_ref, _) = seed_legacy_playback_artifact(&source);

        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen source with legacy playback rows");

        let snapshot = reconcile_readiness_with_cancel_and_progress(
            &connection,
            source.id.as_str(),
            now_epoch_seconds(),
            &AtomicBool::new(false),
            &mut || {},
        )
        .expect("ignore legacy playback rows during reconciliation");
        assert_eq!(snapshot.entries.len(), 4);

        assert!(matches!(
            retire_legacy_playback_readiness(&source, &mut connection, &AtomicBool::new(false))
                .expect("retire legacy playback readiness"),
            Cancellable::Completed(2)
        ));
        let playback_rows = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM analysis_jobs
                     WHERE source_id = ?1
                       AND readiness_managed = 1
                       AND readiness_stage = 'playback_summary')",
                [source.id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .expect("count retired playback rows");
        assert_eq!(playback_rows, 0);
        assert!(!cache_ref.exists());
        assert!(matches!(
            retire_legacy_playback_readiness(&source, &mut connection, &AtomicBool::new(false))
                .expect("repeat legacy retirement"),
            Cancellable::Completed(0)
        ));
    }

    #[test]
    fn post_commit_cancellation_still_retires_every_legacy_cache_ref() {
        let (_directory, source) = ready_analysis_source("legacy-playback-cancellation");
        let database_root = source.database_root().expect("database root");
        let (first_cache_ref, now) = seed_legacy_playback_artifact(&source);
        let second_cache_ref = seed_managed_legacy_cache_ref(&source, "second", now);
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("reopen source with multiple legacy playback rows");
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 )
                 SELECT source_id, scope_kind, 'legacy-second', 'legacy-second.wav', stage,
                        required_version, source_generation, content_generation, eligibility,
                        updated_at
                 FROM source_readiness_targets
                 WHERE source_id = ?1 AND stage = 'playback_summary'",
                [source.id.as_str()],
            )
            .expect("seed second legacy playback target");
        connection
            .execute(
                "INSERT INTO source_readiness_artifacts (
                    source_id, scope_kind, scope_id, relative_path, stage, artifact_version,
                    source_generation, content_generation, artifact_ref, completed_at
                 )
                 SELECT source_id, scope_kind, scope_id, relative_path, stage, required_version,
                        source_generation, content_generation, ?2, ?3
                 FROM source_readiness_targets
                 WHERE source_id = ?1
                   AND scope_id = 'legacy-second'
                   AND stage = 'playback_summary'",
                params![source.id.as_str(), second_cache_ref.to_string_lossy(), now],
            )
            .expect("seed second legacy playback artifact");

        let cancel = AtomicBool::new(false);
        assert!(matches!(
            retire_legacy_playback_readiness_with_post_commit_hook(
                &source,
                &mut connection,
                &cancel,
                || cancel.store(true, Ordering::Release),
            )
            .expect("retire every captured legacy playback reference"),
            Cancellable::Completed(4)
        ));
        assert!(cancelled(&cancel));
        assert!(!first_cache_ref.exists());
        assert!(!second_cache_ref.exists());
    }

    #[test]
    fn legacy_playback_cache_owner_is_retired_after_committed_delete() {
        let (_directory, source) = ready_analysis_source("playback-delete");
        let database_root = source.database_root().expect("database root");
        let (owned_cache_ref, _) = seed_legacy_playback_artifact(&source);

        std::fs::remove_file(source.root.join("ready.wav")).expect("delete source sample");
        let db = source.open_db().expect("open source after delete");
        wavecrate::sample_sources::scanner::sync_paths(&db, &[PathBuf::from("ready.wav")])
            .expect("commit source deletion");
        let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
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
        connection
            .execute(
                "INSERT INTO metadata (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![META_LAST_MANIFEST_AUDIT_AT, now_epoch_seconds().to_string()],
            )
            .expect("mark the fixture manifest audit current");
        drop(connection);

        let mut supervisor =
            SourceProcessingSupervisor::start_without_forced_manifest_audit(vec![source.clone()]);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
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
            if good_hashed && failure_recorded {
                break;
            }
            if Instant::now() >= deadline {
                let telemetry = supervisor.shared.telemetry();
                panic!(
                    "hash fairness did not converge: good_hashed={good_hashed} \
                     unavailable_status={failure_recorded} claimed={} completed={} failed={} \
                     retried={} stale={} queue_depth={}",
                    telemetry.claimed,
                    telemetry.completed,
                    telemetry.failed,
                    telemetry.retried,
                    telemetry.stale,
                    telemetry.queue_depth,
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
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
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
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

        assert!(!source_processing_schema_available(&mut connection).unwrap());
    }

    #[test]
    fn real_hash_execution_waits_for_shared_scan_database_budget() {
        let (_directory, source) = unhashed_source("shared-budget");
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared = Arc::new(Shared::new(vec![source.clone()], Some(Arc::new(sender))));
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
            retirement_worker: None,
        };
        thread::sleep(Duration::from_millis(150));
        assert!(!source_is_hashed(&source));
        assert!(
            supervisor.shared.telemetry().sweeps <= 2,
            "a source blocked by an external scan must park instead of rediscovering in a tight loop"
        );
        assert!(
            receiver
                .try_iter()
                .all(|event| matches!(event, SourceProcessingEvent::Completed)),
            "queued work must not publish active progress while foreground admission owns the lane"
        );

        drop(permit);
        wait_until(Duration::from_secs(3), || source_is_hashed(&source));
        assert_eq!(supervisor.shutdown()["joined"], true);
    }

    #[test]
    fn external_scan_tokens_survive_playback_but_cancel_for_removal_and_shutdown() {
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
        assert!(
            !playback_cancel.load(Ordering::Acquire),
            "playback must not cancel source scanning"
        );
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
            discover_source_candidates(&source, 250, false, &cancel).expect("rediscover work")
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
        assert!(
            readiness.iter().all(|candidate| !matches!(
                candidate.task,
                RuntimeTask::Readiness(ref target)
                    if target.stage == ReadinessStage::SimilarityLayout
            )),
            "similarity layout must stay parked behind the pending embedding target"
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
                   AND readiness_stage IN ('analysis_features', 'embedding_aspects')",
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
            ReadinessMembership::default().generation()
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
        assert!(matches!(
            outcome,
            ExecutionOutcome::PrerequisiteInvalidated {
                reason: "analysis prerequisite artifact payload is missing",
                ..
            }
        ));

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
    fn stale_scope_outcomes_discard_already_discovered_dependents() {
        let indexed = RuntimeTask::Readiness(ReadinessTarget::file(
            "source",
            "identity",
            "changing.wav",
            ReadinessStage::IndexedIdentity,
            "manifest-v1",
            1,
            "pending-identity",
        ));
        let mut analysis_task = indexed.clone();
        let RuntimeTask::Readiness(analysis_target) = &mut analysis_task else {
            unreachable!();
        };
        analysis_target.stage = ReadinessStage::AnalysisFeatures;

        assert_eq!(
            candidate_invalidation_scope(&indexed, Some(ExecutionOutcome::Retried { retry_at: 5 })),
            CandidateInvalidationScope::TargetScope
        );
        assert_eq!(
            candidate_invalidation_scope(&indexed, Some(ExecutionOutcome::Completed)),
            CandidateInvalidationScope::TargetScope,
            "committing a content hash invalidates pending-generation dependents"
        );
        let mut indexed_exact = indexed.clone();
        let RuntimeTask::Readiness(indexed_exact_target) = &mut indexed_exact else {
            unreachable!();
        };
        indexed_exact_target.content_generation = String::from("exact-content-hash");
        assert_eq!(
            candidate_invalidation_scope(&indexed_exact, Some(ExecutionOutcome::Completed)),
            CandidateInvalidationScope::None,
            "recording an already exact indexed identity must preserve same-generation dependents"
        );
        assert_eq!(
            candidate_invalidation_scope(&analysis_task, Some(ExecutionOutcome::Completed)),
            CandidateInvalidationScope::None
        );
        assert_eq!(
            candidate_invalidation_scope(
                &RuntimeTask::ManifestAudit,
                Some(ExecutionOutcome::Completed)
            ),
            CandidateInvalidationScope::Source
        );
    }

    #[test]
    fn retry_only_source_releases_owner_before_another_source_runs() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::default());
        let first = [WorkCandidate::source(
            "recording",
            ProcessingLane::Hashing,
            0,
            0,
        )];
        assert_eq!(
            scheduler.choose(&first, &PriorityContext::default(), &budgets),
            Some(0)
        );

        let configured = ["recording".to_string(), "next".to_string()]
            .into_iter()
            .collect();
        let stats = [(
            "recording".to_string(),
            SourceDiscoveryStats {
                readiness_queue_depth: 1,
                earliest_retry_at: Some(100),
                ..SourceDiscoveryStats::default()
            },
        )]
        .into_iter()
        .collect();
        release_converged_source_owner(&mut scheduler, &configured, &stats, &[]);
        assert_eq!(scheduler.active_source(), None);

        let next = [WorkCandidate::source("next", ProcessingLane::Hashing, 0, 0)];
        assert_eq!(
            scheduler.choose(&next, &PriorityContext::default(), &budgets),
            Some(0)
        );
        assert_eq!(scheduler.active_source(), Some("next"));
    }

    #[test]
    fn discovery_selects_exactly_one_source_and_keeps_the_active_owner() {
        let first = SampleSource::new_with_id(
            SourceId::from_string("first"),
            PathBuf::from("/source/first"),
        );
        let second = SampleSource::new_with_id(
            SourceId::from_string("second"),
            PathBuf::from("/source/second"),
        );
        let sources = vec![first, second];
        let pending = ["first".to_string(), "second".to_string()]
            .into_iter()
            .collect();
        let priority = PriorityContext {
            selected_source: Some("second".to_string()),
            ..PriorityContext::default()
        };

        assert_eq!(
            select_source_for_discovery(&sources, &pending, None, &priority).as_deref(),
            Some("second")
        );
        assert_eq!(
            select_source_for_discovery(&sources, &pending, Some("first"), &priority).as_deref(),
            Some("first"),
            "interactive priority must not switch an active source"
        );
        let pending = ["second".to_string()].into_iter().collect();
        assert_eq!(
            select_source_for_discovery(&sources, &pending, Some("first"), &priority),
            None,
            "another source cannot be discovered while the owner is active"
        );
    }

    #[test]
    fn actively_written_file_is_parked_until_its_quiet_deadline() {
        let (_directory, source) = unhashed_source("active-recording");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        let now = now_epoch_seconds();
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'active-recording-identity',
                     content_hash = NULL,
                     modified_ns = ?1
                 WHERE path = 'pending.wav'",
                [now.saturating_mul(1_000_000_000)],
            )
            .expect("mark file as actively written");
        connection
            .execute(
                "INSERT INTO metadata(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![META_LAST_MANIFEST_AUDIT_AT, now.to_string()],
            )
            .expect("suppress unrelated manifest audit");

        let Cancellable::Completed((candidates, stats)) =
            discover_source_candidates_with_connection(
                &source,
                &mut connection,
                now,
                false,
                &AtomicBool::new(false),
            )
            .expect("discover active recording")
        else {
            panic!("active recording discovery cancelled");
        };
        assert!(
            candidates
                .iter()
                .all(|candidate| { candidate.schedule.scope_id != "active-recording-identity" }),
            "no readiness stage may run while the file is still changing"
        );
        assert!(
            stats
                .earliest_retry_at
                .is_some_and(|retry_at| retry_at > now)
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM analysis_jobs
                     WHERE readiness_managed = 1
                       AND readiness_scope_id = 'active-recording-identity'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count parked jobs"),
            0
        );

        let stable_now = now.saturating_add(ACTIVE_RECORDING_QUIET_SECONDS + 1);
        let Cancellable::Completed((candidates, _)) = discover_source_candidates_with_connection(
            &source,
            &mut connection,
            stable_now,
            false,
            &AtomicBool::new(false),
        )
        .expect("rediscover stable recording") else {
            panic!("stable recording discovery cancelled");
        };
        assert!(candidates.iter().any(|candidate| {
            candidate.schedule.scope_id == "active-recording-identity"
                && candidate.schedule.lane == ProcessingLane::Hashing
        }));
    }

    #[test]
    fn recently_modified_hashed_wav_with_subsecond_mtime_is_parked_until_quiet() {
        let (_directory, source) = unhashed_source("hashed-active-recording");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open readiness database");
        let now = now_epoch_seconds();
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'hashed-active-recording-identity',
                     content_hash = 'already-hashed-recording-content',
                     modified_ns = ?1
                 WHERE path = 'pending.wav'",
                [now.saturating_mul(1_000_000_000)
                    .saturating_add(500_000_000)],
            )
            .expect("mark hashed file as actively written");
        connection
            .execute(
                "INSERT INTO metadata(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![META_LAST_MANIFEST_AUDIT_AT, now.to_string()],
            )
            .expect("suppress unrelated manifest audit");

        let Cancellable::Completed((candidates, stats)) =
            discover_source_candidates_with_connection(
                &source,
                &mut connection,
                now,
                false,
                &AtomicBool::new(false),
            )
            .expect("discover hashed active recording")
        else {
            panic!("hashed active recording discovery cancelled");
        };
        assert!(
            candidates.iter().all(|candidate| {
                candidate.schedule.scope_id != "hashed-active-recording-identity"
            }),
            "an already-hashed WAV must not enter downstream readiness while recently modified"
        );
        assert!(
            stats
                .earliest_retry_at
                .is_some_and(|retry_at| retry_at > now),
            "the quiet deadline must keep the source scheduled for retry"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM analysis_jobs
                     WHERE readiness_managed = 1
                       AND readiness_scope_id = 'hashed-active-recording-identity'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count parked hashed recording jobs"),
            0
        );

        let stable_now = now.saturating_add(ACTIVE_RECORDING_QUIET_SECONDS + 1);
        let Cancellable::Completed((candidates, _)) = discover_source_candidates_with_connection(
            &source,
            &mut connection,
            stable_now,
            false,
            &AtomicBool::new(false),
        )
        .expect("rediscover stable hashed recording") else {
            panic!("stable hashed recording discovery cancelled");
        };
        assert!(candidates.iter().any(|candidate| {
            candidate.schedule.scope_id == "hashed-active-recording-identity"
                && candidate.schedule.lane == ProcessingLane::FeatureAnalysis
        }));
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
    fn typed_decoder_failure_persists_unsupported_code_without_text_classification() {
        let (_directory, source) = unhashed_source("typed-decoder-failure");
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open source database");
        connection
            .execute(
                "UPDATE wav_files
                 SET file_identity = 'typed-decoder-identity', content_hash = 'typed-decoder-hash'
                 WHERE path = 'pending.wav'",
                [],
            )
            .expect("set typed decoder identity");
        let indexed_target = ReadinessTarget::file(
            source.id.as_str(),
            "typed-decoder-identity",
            "pending.wav",
            ReadinessStage::IndexedIdentity,
            "manifest-v1",
            1,
            "typed-decoder-hash",
        );
        let target = ReadinessTarget::file(
            source.id.as_str(),
            "typed-decoder-identity",
            "pending.wav",
            ReadinessStage::AnalysisFeatures,
            "analysis-v1",
            1,
            "typed-decoder-hash",
        );
        let mut embedding_target = target.clone();
        embedding_target.stage = ReadinessStage::EmbeddingAspects;
        embedding_target.eligibility = ReadinessEligibility::Unsupported;
        let similarity_target = ReadinessTarget::source(
            source.id.as_str(),
            ReadinessStage::SimilarityLayout,
            "layout-v1",
            1,
            "members-1",
        )
        .with_eligibility(ReadinessEligibility::Unsupported);
        let now = now_epoch_seconds();
        replace_readiness_targets(
            &mut connection,
            source.id.as_str(),
            1,
            1,
            SourceAvailability::Active,
            &[
                indexed_target,
                target.clone(),
                embedding_target,
                similarity_target,
            ],
            now,
        )
        .expect("publish readiness target");
        let snapshot = reconcile_readiness(&connection, source.id.as_str(), now)
            .expect("reconcile readiness target");
        persist_readiness_deficits(&mut connection, &snapshot.deficits, now)
            .expect("persist readiness target");
        let claim = claim_readiness_target(&mut connection, &target, now, 30)
            .expect("claim readiness target")
            .expect("target claimed");

        let failure = SourceProcessingFailure::from(
            wavecrate::readiness_execution::ReadinessStageError::Decode(
                wavecrate_analysis::AnalysisDecodeError::Unsupported(
                    "wrapped decoder wording must not affect policy".to_string(),
                ),
            ),
        );
        let policy =
            ReadinessRetryPolicy::new(5, 300, READINESS_MAX_ATTEMPTS).expect("valid retry policy");
        assert_eq!(
            ReadinessStore::new(&mut connection)
                .fail(
                    &claim,
                    failure.readiness_failure_classification(),
                    failure.code.as_str(),
                    &failure.context,
                    now,
                    policy,
                )
                .expect("persist typed decoder failure"),
            ReadinessFailureOutcome::Unsupported
        );
        let stored = connection
            .query_row(
                "SELECT failure_kind, failure_code, last_error
                 FROM analysis_jobs
                 WHERE source_id = ?1",
                [claim.target.source_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("read persisted typed failure");
        assert_eq!(
            stored,
            (
                "unsupported".to_string(),
                "decoder_unsupported".to_string(),
                "Audio codec is unsupported".to_string(),
            )
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
                    failure_code TEXT,
                    retry_at INTEGER,
                    last_error TEXT
                );
                INSERT INTO analysis_jobs VALUES
                    (1, 'source', 1, 'file', 'bad-audio', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500,
                        'failed to decode audio file: Invalid wav'),
                    (2, 'source', 1, 'file', 'transient', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500, 'database is locked'),
                    (3, 'source', 1, 'file', 'pending', 'analysis_features', 'hash',
                        'pending', 0, NULL, NULL, NULL, NULL),
                    (4, 'source', 0, 'file', 'legacy', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500, 'unsupported codec'),
                    (5, 'source', 1, 'file', 'bad-audio', 'embedding_aspects', 'hash',
                        'failed', 3, 'retryable', NULL, 500,
                        'embedding feature prerequisite is not durable yet'),
                    (6, 'source', 1, 'file', 'missing-payload', 'embedding_aspects', 'hash',
                        'failed', 8, 'permanent', NULL, NULL,
                        'embedding feature prerequisite is not durable yet'),
                    (7, 'source', 1, 'file', 'legacy-permanent', 'analysis_features', 'hash',
                        'failed', 8, 'permanent', NULL, NULL,
                        'Audio decode failed for empty.wav: no suitable format reader found'),
                    (8, 'source', 1, 'file', 'current-coded', 'analysis_features', 'hash',
                        'failed', 1, 'permanent', 'execution_unclassified', NULL,
                        'Audio decode failed for current.wav: no suitable format reader found');",
            )
            .expect("seed readiness failures");

        assert_eq!(
            reclassify_known_unsupported_audio_failures(&mut connection)
                .expect("reclassify unsupported failures"),
            3
        );
        let first = connection
            .query_row(
                "SELECT failure_kind, failure_code, retry_at FROM analysis_jobs WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .expect("read reclassified failure");
        assert_eq!(
            first,
            (
                String::from("unsupported"),
                Some(String::from("legacy_decoder_unsupported")),
                None
            )
        );
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
        let current_coded = connection
            .query_row(
                "SELECT failure_kind, failure_code, retry_at FROM analysis_jobs WHERE id = 8",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .expect("read current coded failure");
        assert_eq!(
            current_coded,
            (
                String::from("permanent"),
                Some(String::from("execution_unclassified")),
                None,
            )
        );
        let exhausted = connection
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
            .expect("read exhausted prerequisite failure");
        assert_eq!(exhausted, (String::from("permanent"), 8, None));
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

    fn seed_legacy_playback_artifact(source: &SampleSource) -> (std::path::PathBuf, i64) {
        let now = now_epoch_seconds();
        let database_root = source.database_root().expect("database root");
        let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .expect("open legacy playback database");
        publish_current_readiness_targets(&mut connection, source.id.as_str(), now)
            .expect("publish current target matrix");
        let cache_ref = seed_managed_legacy_cache_ref(source, "first", now);
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 )
                 SELECT source_id, scope_kind, scope_id, relative_path, 'playback_summary',
                        'legacy-playback-v1', source_generation, content_generation,
                        eligibility, ?2
                 FROM source_readiness_targets
                 WHERE source_id = ?1 AND stage = 'indexed_identity'",
                params![source.id.as_str(), now],
            )
            .expect("seed legacy playback target");
        connection
            .execute(
                "INSERT INTO source_readiness_artifacts (
                    source_id, scope_kind, scope_id, relative_path, stage, artifact_version,
                    source_generation, content_generation, artifact_ref, completed_at
                 )
                 SELECT source_id, scope_kind, scope_id, relative_path, stage, required_version,
                        source_generation, content_generation, ?2, ?3
                 FROM source_readiness_targets
                 WHERE source_id = ?1 AND stage = 'playback_summary'",
                params![source.id.as_str(), cache_ref.to_string_lossy(), now],
            )
            .expect("seed legacy playback artifact");
        (cache_ref, now)
    }

    fn seed_managed_legacy_cache_ref(
        source: &SampleSource,
        label: &str,
        now: i64,
    ) -> std::path::PathBuf {
        let cache_directory =
            wavecrate::app_dirs::waveform_cache_dir().expect("resolve waveform cache directory");
        std::fs::create_dir_all(&cache_directory).expect("create waveform cache directory");
        let cache_ref = cache_directory.join(format!(
            "legacy-playback-{}-{label}-{now}.wfc",
            source.id.as_str()
        ));
        std::fs::write(&cache_ref, b"legacy playback cache").expect("seed legacy playback cache");
        cache_ref
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
                    (SELECT COUNT(*) FROM source_readiness_sources
                     WHERE contract_version != '')",
                [],
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
