use crate::app::controller::jobs::SourceHydrationKind;
use crate::app::state::FolderPaneId;
use crate::sample_sources::Rating;
use crate::sample_sources::SourceId;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

/// Source-lane runtime state grouped by hydration, folder projection, and mutations.
#[derive(Debug, Default)]
pub(crate) struct SourceLaneRuntimeState {
    /// In-flight source hydration requests keyed by selection lane.
    pub(crate) hydration: SourceHydrationRuntime,
    /// Pending pane-scoped folder projection work.
    pub(crate) folder_projection: FolderProjectionRuntime,
    /// Background metadata/file mutation tracking used by optimistic UI state.
    pub(crate) mutations: SourceMutationRuntime,
}

/// Runtime tracking for active and inactive source hydration requests.
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceHydrationRuntime {
    /// Active source hydration currently preparing the browser-driving source snapshot.
    pub(crate) pending_active: Option<PendingSourceHydration>,
    /// Inactive-pane source hydration currently preparing one retained folder snapshot.
    pub(crate) pending_inactive: Option<PendingSourceHydration>,
}

impl SourceHydrationRuntime {
    /// Return the pending hydration request for `kind`, when one exists.
    pub(crate) fn pending(&self, kind: SourceHydrationKind) -> Option<&PendingSourceHydration> {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active.as_ref(),
            SourceHydrationKind::InactivePane => self.pending_inactive.as_ref(),
        }
    }

    /// Return the mutable pending hydration request for `kind`, when one exists.
    pub(crate) fn pending_mut(
        &mut self,
        kind: SourceHydrationKind,
    ) -> Option<&mut PendingSourceHydration> {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active.as_mut(),
            SourceHydrationKind::InactivePane => self.pending_inactive.as_mut(),
        }
    }

    /// Store a new pending hydration request in the lane identified by `kind`.
    pub(crate) fn set_pending(
        &mut self,
        kind: SourceHydrationKind,
        pending: PendingSourceHydration,
    ) {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active = Some(pending),
            SourceHydrationKind::InactivePane => self.pending_inactive = Some(pending),
        }
    }

    /// Clear the pending hydration request for `kind`.
    pub(crate) fn clear_pending(&mut self, kind: SourceHydrationKind) {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active = None,
            SourceHydrationKind::InactivePane => self.pending_inactive = None,
        }
    }

    /// Return the source id currently owning a visible loading state, when any.
    pub(crate) fn loading_source_id(&self) -> Option<SourceId> {
        self.pending_active
            .as_ref()
            .map(|pending| pending.source_id.clone())
            .or_else(|| {
                self.pending_inactive
                    .as_ref()
                    .map(|pending| pending.source_id.clone())
            })
    }
}

/// Runtime tracking for pane-scoped folder projection requests.
#[derive(Clone, Debug, Default)]
pub(crate) struct FolderProjectionRuntime {
    /// Pending pane-scoped folder projection jobs keyed by owning sidebar pane.
    pub(crate) pending: HashMap<FolderPaneId, PendingFolderProjection>,
}

/// Runtime tracking for optimistic metadata writes and background file mutations.
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceMutationRuntime {
    /// Controller-owned metadata writes awaiting background completion by request id.
    pending_metadata_mutations: HashMap<u64, PendingMetadataMutation>,
    /// Relative sample paths currently carrying optimistic metadata writes.
    pending_metadata_paths: HashSet<(SourceId, PathBuf)>,
    /// Relative sample paths whose optimistic metadata writes should block
    /// browser file mutations until the source metadata commit settles.
    pending_file_blocking_metadata_paths: HashSet<(SourceId, PathBuf)>,
    /// Short source-scoped grace window that keeps analysis claiming paused
    /// across adjacent quick edits on the same selected source.
    claim_pause_grace_until: HashMap<SourceId, Instant>,
    /// Source ids currently owning background file or folder mutations.
    pending_file_mutation_sources: HashSet<SourceId>,
    /// Relative paths currently carrying background file or folder mutations.
    pending_file_mutation_paths: HashSet<(SourceId, PathBuf)>,
}

impl SourceMutationRuntime {
    /// Track one optimistic metadata request and its affected relative paths.
    pub(crate) fn insert_metadata_mutation(&mut self, pending: PendingMetadataMutation) {
        let source_id = pending.source_id.clone();
        for path in &pending.paths {
            self.pending_metadata_paths
                .insert((source_id.clone(), path.clone()));
            if pending.blocks_file_mutation {
                self.pending_file_blocking_metadata_paths
                    .insert((source_id.clone(), path.clone()));
            }
        }
        self.pending_metadata_mutations
            .insert(pending.request_id, pending);
    }

    /// Remove one metadata request and clear its tracked pending paths.
    pub(crate) fn finish_metadata_mutation(
        &mut self,
        request_id: u64,
    ) -> Option<PendingMetadataMutation> {
        let pending = self.pending_metadata_mutations.remove(&request_id)?;
        for path in &pending.paths {
            self.pending_metadata_paths
                .remove(&(pending.source_id.clone(), path.clone()));
            if pending.blocks_file_mutation {
                self.pending_file_blocking_metadata_paths
                    .remove(&(pending.source_id.clone(), path.clone()));
            }
        }
        Some(pending)
    }

    /// Return whether one sample path still has an optimistic metadata write in flight.
    pub(crate) fn metadata_path_pending(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
    ) -> bool {
        self.pending_file_blocking_metadata_paths
            .contains(&(source_id.clone(), relative_path.to_path_buf()))
    }

    /// Return whether any optimistic metadata write is pending for `source_id`.
    pub(crate) fn source_has_pending_metadata(&self, source_id: &SourceId) -> bool {
        self.pending_metadata_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id)
    }

    /// Extend the source-scoped claim-pause grace window through `until`.
    pub(crate) fn extend_claim_pause_grace(&mut self, source_id: &SourceId, until: Instant) {
        let entry = self
            .claim_pause_grace_until
            .entry(source_id.clone())
            .or_insert(until);
        if *entry < until {
            *entry = until;
        }
    }

    /// Return whether the source-scoped claim-pause grace window is still active.
    pub(crate) fn claim_pause_grace_active(&mut self, source_id: &SourceId, now: Instant) -> bool {
        match self.claim_pause_grace_until.get(source_id).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                self.claim_pause_grace_until.remove(source_id);
                false
            }
            None => false,
        }
    }

    /// Return whether one source currently owns a background file mutation.
    pub(crate) fn source_has_pending_file_mutations(&self, source_id: &SourceId) -> bool {
        self.pending_file_mutation_sources.contains(source_id)
    }

    /// Mark one source/path batch as owned by a background file mutation.
    pub(crate) fn begin_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        self.pending_file_mutation_sources.insert(source_id.clone());
        for path in paths {
            self.pending_file_mutation_paths
                .insert((source_id.clone(), path));
        }
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        for path in paths {
            self.pending_file_mutation_paths
                .remove(&(source_id.clone(), path));
        }
        let still_has_paths = self
            .pending_file_mutation_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id);
        if !still_has_paths {
            self.pending_file_mutation_sources.remove(source_id);
        }
    }
}

/// Active controller-side tracking for one source hydration request.
#[derive(Clone, Debug)]
pub(crate) struct PendingSourceHydration {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane that owns the source assignment.
    pub(crate) pane: FolderPaneId,
    /// Hydrated source identifier.
    pub(crate) source_id: SourceId,
    /// Logical hydration lane for result application.
    pub(crate) kind: SourceHydrationKind,
    /// Search request queued after hydration apply, when active-source projection is pending.
    pub(crate) search_request_id: Option<u64>,
    /// Time when the hydration request was queued on the controller thread.
    pub(crate) queued_at: Instant,
}

/// Active controller-side tracking for one pane-scoped folder projection request.
#[derive(Clone, Debug)]
pub(crate) struct PendingFolderProjection {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane whose folder browser rows are being projected.
    pub(crate) pane: FolderPaneId,
    /// Source identifier that owns the folder browser state.
    pub(crate) source_id: SourceId,
    /// Time when the projection request was queued on the controller thread.
    pub(crate) queued_at: Instant,
}

/// One optimistic metadata request awaiting background persistence.
#[derive(Clone, Debug)]
pub(crate) struct PendingMetadataMutation {
    /// Request id used to match completion messages.
    pub(crate) request_id: u64,
    /// Source that owns the optimistic metadata updates.
    pub(crate) source_id: SourceId,
    /// Paths touched by this request for pending-state cleanup.
    pub(crate) paths: BTreeSet<PathBuf>,
    /// Whether this mutation must settle before browser file mutations like
    /// rename/auto-rename are allowed to proceed.
    pub(crate) blocks_file_mutation: bool,
    /// Rollback entries applied only when the background write fails.
    pub(crate) rollback: Vec<MetadataRollback>,
    /// Whether the browser filter/sort projection should refresh when the write completes.
    pub(crate) refresh_browser_projection: bool,
}

/// Rollback payload for one optimistic metadata update.
#[derive(Clone, Debug)]
pub(crate) enum MetadataRollback {
    /// Restore one tag plus keep-lock state if the optimistic value is still current.
    TagAndLocked {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_tag: Rating,
        /// Lock state before the optimistic mutation.
        before_locked: bool,
        /// Value written optimistically before persistence completed.
        expected_tag: Rating,
        /// Lock state written optimistically before persistence completed.
        expected_locked: bool,
    },
    /// Restore one loop-marker state if the optimistic value is still current.
    Looped {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_looped: bool,
        /// Value written optimistically before persistence completed.
        expected_looped: bool,
    },
    /// Restore one sound-type value if the optimistic value is still current.
    SoundType {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_sound_type: Option<crate::sample_sources::SampleSoundType>,
        /// Value written optimistically before persistence completed.
        expected_sound_type: Option<crate::sample_sources::SampleSoundType>,
    },
    /// Restore one custom user-tag value if the optimistic value is still current.
    UserTag {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_user_tag: Option<String>,
        /// Value written optimistically before persistence completed.
        expected_user_tag: Option<String>,
    },
    /// Restore one playback-age value if the optimistic value is still current.
    LastPlayedAt {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_last_played_at: Option<i64>,
        /// Value written optimistically before persistence completed.
        expected_last_played_at: Option<i64>,
    },
    /// Restore one BPM value if the optimistic value is still current.
    Bpm {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_bpm: Option<f32>,
        /// Value written optimistically before persistence completed.
        expected_bpm: Option<f32>,
    },
}
