use super::{
    Arc, AtomicBool, DatabasePhase, DatabaseWriterGuard, ExternalScanAdmission,
    ExternalScanRegistration, Ordering, PathBuf, ProcessingLane, SampleSource, Shared,
    resolve_registered_source_for_scan_locked,
};

#[derive(Clone)]
pub(in crate::native_app) struct SourceProcessingBudgetHandle {
    pub(super) shared: Arc<Shared>,
}

pub(in crate::native_app) struct SourceProcessingBudgetPermit {
    shared: Arc<Shared>,
    pub(super) permit: Option<super::super::scheduler::BudgetPermit>,
    database_writer: Option<DatabaseWriterGuard>,
    registration_id: u64,
    pub(super) lifecycle_generation: u64,
    pub(super) cancel: Arc<AtomicBool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourceScanAdmissionState {
    WaitingForSourceActivation,
    WaitingForCapacity { current_owner: Option<String> },
    WaitingForDatabaseAccess,
    Admitted,
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
        self.acquire_scan_for_generation_with_state(
            source_id,
            expected_lifecycle_generation,
            |_| {},
        )
    }

    pub(in crate::native_app) fn acquire_scan_for_generation_with_state(
        &self,
        source_id: &str,
        expected_lifecycle_generation: u64,
        mut publish_state: impl FnMut(SourceScanAdmissionState),
    ) -> Option<SourceProcessingBudgetPermit> {
        {
            let mut control = self.shared.control();
            let mut waiting_published = false;
            while !control.shutdown
                && control.source_is_configured(source_id)
                && control.source_lifecycle_generations.get(source_id)
                    == Some(&expected_lifecycle_generation)
                && !control.source_is_active(source_id)
            {
                if !waiting_published {
                    publish_state(SourceScanAdmissionState::WaitingForSourceActivation);
                    waiting_published = true;
                }
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
        let mut capacity_wait_published = false;
        loop {
            let mut budgets = self.shared.budgets();
            if let Some(permit) = budgets.try_acquire(source_id, ProcessingLane::Scan) {
                drop(budgets);
                publish_state(SourceScanAdmissionState::WaitingForDatabaseAccess);
                let database_writer = self
                    .shared
                    .database_writer
                    .lock(DatabasePhase::SerialCompatibility);
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
                    drop(database_writer);
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
                    database_writer: Some(database_writer),
                    registration_id: admission_id,
                    lifecycle_generation,
                    cancel,
                };
                if permit.should_cancel_now() {
                    permit.cancel.store(true, Ordering::Release);
                }
                publish_state(SourceScanAdmissionState::Admitted);
                return Some(permit);
            }
            if !capacity_wait_published {
                let current_owner = budgets
                    .active_sources()
                    .into_iter()
                    .find(|active_source| active_source != source_id);
                publish_state(SourceScanAdmissionState::WaitingForCapacity { current_owner });
                capacity_wait_published = true;
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
        self.database_writer.take();
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

pub(super) fn install_worker_app_root(app_root: PathBuf) -> wavecrate::app_dirs::AppRootGuard {
    wavecrate::app_dirs::AppRootGuard::set(app_root)
        .expect("source-processing worker should inherit the resolved persistence root")
}
