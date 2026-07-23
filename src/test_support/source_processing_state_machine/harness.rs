use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use wavecrate_scan::{
    ScanError, ScanMode, complete_deferred_hashes, scan_with_progress, sync_paths,
};

use crate::{
    native_source_fixture::{FixtureName, FixtureProfile, FixtureProvisionRequest, provision},
    sample_sources::{SampleSource, SourceId},
};

use super::super::{SourceAuditLifecycleCause, SourceProcessingSupervisor};
use super::{
    Event, FailureBoundary, FailureSnapshot, ReferenceModel, ScanCause,
    invariants::filesystem_inventory,
};

pub(super) struct StateMachineHarness {
    pub(super) _config_base: tempfile::TempDir,
    pub(super) source: SampleSource,
    pub(super) escape_target: PathBuf,
    pub(super) model: ReferenceModel,
    pub(super) supervisor: Option<SourceProcessingSupervisor>,
    pub(super) template: Vec<u8>,
    pub(super) last_revision: u64,
    pub(super) accepted_publications: BTreeSet<(u64, u64, ScanCause)>,
    pub(super) accepted_revisions: Vec<String>,
    pub(super) next_failure: Option<FailureBoundary>,
    pub(super) retired_roots: Vec<PathBuf>,
    pub(super) observable_commits: u64,
}

impl StateMachineHarness {
    pub(super) fn new(with_supervisor: bool) -> Result<Self, String> {
        let config_base = tempfile::tempdir().map_err(|error| error.to_string())?;
        let manifest = provision(&FixtureProvisionRequest {
            config_base: config_base.path().to_path_buf(),
            fixture: FixtureName::SmallMultiSource,
            profile: FixtureProfile::AutomatedTests,
            reset: true,
        })?;
        let source_manifest = manifest
            .sources
            .iter()
            .find(|source| source.directory_name == "source-beta")
            .ok_or_else(|| String::from("small fixture is missing source-beta"))?;
        let escape_target = manifest
            .sources
            .iter()
            .find(|source| source.directory_name == "source-alpha")
            .map(|source| source.root.clone())
            .ok_or_else(|| String::from("small fixture is missing source-alpha"))?;
        let source = SampleSource::new_with_id(
            SourceId::from_string(source_manifest.source_id.clone()),
            source_manifest.root.clone(),
        );
        let template = fs::read(source.root.join("mutable/change-me.wav"))
            .map_err(|error| format!("read deterministic mutation template: {error}"))?;
        let files = filesystem_inventory(&source.root)?;
        let database = source.open_db().map_err(|error| error.to_string())?;
        let last_revision = database
            .get_wav_paths_revision()
            .map_err(|error| error.to_string())?;
        let supervisor =
            with_supervisor.then(|| SourceProcessingSupervisor::start(vec![source.clone()]));
        Ok(Self {
            _config_base: config_base,
            source,
            escape_target,
            model: ReferenceModel::new(files),
            supervisor,
            template,
            last_revision,
            accepted_publications: BTreeSet::new(),
            accepted_revisions: vec![format!("1:{last_revision}")],
            next_failure: None,
            retired_roots: Vec::new(),
            observable_commits: 0,
        })
    }

    pub(super) fn run(mut self, events: &[Event]) -> Result<(), FailureSnapshot> {
        for (event_index, event) in events.iter().enumerate() {
            if let Err(message) = self.apply(event) {
                return Err(self.failure(event_index, event, message));
            }
        }
        if let Err(message) = self.quiesce() {
            return Err(self.failure(events.len(), &Event::Quiesce, message));
        }
        if let Some(mut supervisor) = self.supervisor.take() {
            let report = supervisor.shutdown();
            if report["joined"] != true {
                return Err(self.failure(
                    events.len(),
                    &Event::Quiesce,
                    String::from("source-processing supervisor did not join"),
                ));
            }
        }
        Ok(())
    }

    fn apply(&mut self, event: &Event) -> Result<(), String> {
        match *event {
            Event::Create { slot } => self.create(slot, false),
            Event::SameSizeModify { slot } => self.modify(slot),
            Event::Move { slot, nested } => self.move_file(slot, nested),
            Event::Delete { slot } => self.delete(slot),
            Event::NestedDirectoryChange { slot } => self.create(slot, true),
            Event::WatcherBatch => self.flush(ScanCause::Watcher),
            Event::WatcherOverflow => {
                self.model.queue(ScanCause::WatcherOverflow);
                self.flush(ScanCause::WatcherOverflow)
            }
            Event::FocusChanged { active } => {
                self.model.focused = active;
                if let Some(supervisor) = &self.supervisor {
                    supervisor.set_foreground_activity(!active);
                    if active {
                        supervisor.request_lifecycle_audit_probe(
                            SourceAuditLifecycleCause::FocusRegained,
                            &[],
                        );
                    }
                }
                if active {
                    self.model.queue(ScanCause::Focus);
                }
                self.assert_queue_bound()
            }
            Event::ExplicitRefresh => {
                self.model.queue(ScanCause::Foreground);
                self.flush(ScanCause::Foreground)
            }
            Event::Cancel => self.cancel_scan(),
            Event::ShutdownRestart => self.shutdown_restart(),
            Event::SourceRemoveReadd => self.remove_readd(),
            Event::RootOfflineOnline => self.root_offline_online(),
            Event::RootReplacement => self.root_replacement(),
            Event::PartialEnumeration => self.partial_enumeration(),
            Event::SymlinkEscape => self.symlink_escape(),
            Event::DatabaseBusy => {
                self.next_failure = Some(FailureBoundary::Transaction);
                self.flush(ScanCause::Watcher)
            }
            Event::InjectFailure { boundary } => {
                self.next_failure = Some(boundary);
                Ok(())
            }
            Event::Quiesce => self.quiesce(),
        }
    }

    pub(super) fn flush(&mut self, cause: ScanCause) -> Result<(), String> {
        if !self.model.root_online || !self.model.source_configured {
            self.model.queue(ScanCause::Retry);
            return Ok(());
        }
        let effective_cause = self.authoritative_cause(cause);
        if self.take_failure(FailureBoundary::Transaction) {
            self.model.retry_count = self.model.retry_count.saturating_add(1);
            self.model.queue(ScanCause::Retry);
            return Ok(());
        }
        if cause == ScanCause::Watcher && self.take_failure(FailureBoundary::WatcherDelivery) {
            self.model.retry_count = self.model.retry_count.saturating_add(1);
            self.model.queue(ScanCause::Retry);
            return Ok(());
        }
        let database = self.database()?;
        let pending_paths = self
            .model
            .watcher_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let stats = if self.take_failure(FailureBoundary::Hashing) {
            let cancel = AtomicBool::new(false);
            let mut progressed = false;
            let result =
                scan_with_progress(&database, ScanMode::Quick, Some(&cancel), &mut |_, _| {
                    if progressed {
                        cancel.store(true, Ordering::Release);
                    }
                    progressed = true;
                });
            self.model.queue(ScanCause::Retry);
            self.model.retry_count = self.model.retry_count.saturating_add(1);
            match result {
                Ok(stats) => stats,
                Err(ScanError::Incomplete { committed, .. }) => *committed,
                Err(ScanError::Canceled) => return Ok(()),
                Err(error) => return Err(format!("hash-boundary scan failed: {error}")),
            }
        } else if effective_cause == ScanCause::Watcher && !pending_paths.is_empty() {
            sync_paths(&database, &pending_paths)
                .map_err(|error| format!("targeted watcher reconciliation failed: {error}"))?
        } else {
            scan_with_progress(&database, ScanMode::Quick, None, &mut |_, _| {})
                .map_err(|error| format!("full reconciliation failed: {error}"))?
        };
        let stats = complete_deferred_hashes(&database, stats)
            .map_err(|error| format!("complete deferred hashes: {error}"))?;
        let publication_lost = self.take_failure(FailureBoundary::Publication);
        self.accept_commit(effective_cause, &stats.committed_delta, publication_lost)?;
        self.model.watcher_paths.clear();
        if effective_cause == ScanCause::Watcher {
            self.model.queued_causes.remove(&ScanCause::Watcher);
        } else {
            self.model.queued_causes.clear();
        }
        if publication_lost {
            self.model.queue(ScanCause::Retry);
            self.model.retry_count = self.model.retry_count.saturating_add(1);
        }
        if let Some(supervisor) = &self.supervisor {
            if stats.committed_delta.is_empty() {
                supervisor
                    .request_source_processing(self.source.id.as_str(), "state_machine_scan_noop");
            } else {
                supervisor.request_source_delta(
                    self.source.id.as_str(),
                    &stats.committed_delta,
                    "state_machine_scan_commit",
                );
            }
        }
        self.assert_committed_manifest(&database)
    }

    fn authoritative_cause(&self, requested: ScanCause) -> ScanCause {
        [
            ScanCause::Lifecycle,
            ScanCause::Restart,
            ScanCause::WatcherOverflow,
            ScanCause::Foreground,
            ScanCause::Focus,
            ScanCause::Retry,
        ]
        .into_iter()
        .find(|cause| self.model.queued_causes.contains(cause))
        .unwrap_or(requested)
    }

    fn quiesce(&mut self) -> Result<(), String> {
        self.require_online()?;
        self.next_failure = None;
        self.model.queue(ScanCause::Retry);
        self.flush(ScanCause::Retry)?;
        self.model.queued_causes.clear();
        self.model.watcher_paths.clear();
        let database = self.database()?;
        self.assert_committed_manifest(&database)?;
        if let Some(supervisor) = &self.supervisor {
            supervisor.wake_source_for_full_reconciliation(
                self.source.id.as_str(),
                "state_machine_quiescence",
            );
            self.assert_runtime_liveness(supervisor)?;
        }
        Ok(())
    }
}
