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
    /// Short source-scoped grace window that suppresses auto sync after
    /// controller-owned file mutations already updated the browser state.
    auto_sync_grace_until: HashMap<SourceId, Instant>,
    /// Source ids currently owning background file or folder mutations.
    pending_file_mutation_sources: HashSet<SourceId>,
    /// Relative paths currently carrying background file or folder mutations.
    pending_file_mutation_paths: HashSet<(SourceId, PathBuf)>,
    /// Latest optimistic Loop/One-shot edit owner for each source/path.
    looped_metadata_intents: HashMap<(SourceId, PathBuf), u64>,
    /// Monotonic id used to make Loop/One-shot rollback latest-intent aware.
    next_looped_metadata_intent_id: u64,
    /// Active browser rename/auto-rename request currently owning the file-op lane.
    active_browser_rename_intent: Option<BrowserRenameIntentKey>,
    /// One deferred browser auto-rename request captured while browser rename work is active.
    queued_browser_auto_rename_intent: Option<PendingBrowserAutoRenameIntent>,
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

    #[cfg(test)]
    /// Return the number of metadata mutation requests currently awaiting completion.
    pub(crate) fn pending_metadata_count(&self) -> usize {
        self.pending_metadata_mutations.len()
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

    /// Extend the source-scoped auto-sync suppression window through `until`.
    pub(crate) fn extend_auto_sync_grace(&mut self, source_id: &SourceId, until: Instant) {
        let entry = self
            .auto_sync_grace_until
            .entry(source_id.clone())
            .or_insert(until);
        if *entry < until {
            *entry = until;
        }
    }

    /// Return whether the source-scoped auto-sync suppression window is active.
    pub(crate) fn auto_sync_grace_active(&mut self, source_id: &SourceId, now: Instant) -> bool {
        match self.auto_sync_grace_until.get(source_id).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                self.auto_sync_grace_until.remove(source_id);
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
    ) -> bool {
        let source_was_inactive = self.pending_file_mutation_sources.insert(source_id.clone());
        for path in paths {
            self.pending_file_mutation_paths
                .insert((source_id.clone(), path));
        }
        source_was_inactive
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> bool {
        for path in paths {
            self.pending_file_mutation_paths
                .remove(&(source_id.clone(), path));
        }
        let still_has_paths = self
            .pending_file_mutation_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id);
        if !still_has_paths {
            return self.pending_file_mutation_sources.remove(source_id);
        }
        false
    }

    /// Mark one browser rename or auto-rename intent as the file-op lane owner.
    pub(crate) fn begin_browser_rename_intent(&mut self, key: BrowserRenameIntentKey) {
        self.active_browser_rename_intent = Some(key);
    }

    /// Drop the active browser rename owner and return any queued follow-up.
    pub(crate) fn finish_browser_rename_intent(
        &mut self,
    ) -> Option<PendingBrowserAutoRenameIntent> {
        self.active_browser_rename_intent = None;
        self.queued_browser_auto_rename_intent.take()
    }

    /// Forget active browser rename ownership when dispatch fails before work starts.
    pub(crate) fn clear_browser_rename_intent(&mut self) {
        self.active_browser_rename_intent = None;
    }

    /// Apply the OPT-135 policy for repeat auto-rename input while a file op is active.
    pub(crate) fn handle_busy_browser_auto_rename_intent(
        &mut self,
        key: BrowserRenameIntentKey,
        pending: PendingBrowserAutoRenameIntent,
    ) -> BrowserRenameBusyDecision {
        let Some(active) = self.active_browser_rename_intent.as_ref() else {
            return BrowserRenameBusyDecision::UnrelatedFileOp;
        };
        if active == &key
            || self
                .queued_browser_auto_rename_intent
                .as_ref()
                .is_some_and(|queued| queued.key == key)
        {
            return BrowserRenameBusyDecision::Collapsed;
        }
        self.queued_browser_auto_rename_intent = Some(pending);
        BrowserRenameBusyDecision::Queued
    }

    /// Return whether the same browser rename request is already active.
    pub(crate) fn browser_rename_intent_is_active(&self, key: &BrowserRenameIntentKey) -> bool {
        self.active_browser_rename_intent.as_ref() == Some(key)
    }

    /// Record the latest optimistic Loop/One-shot edit for one source/path.
    pub(crate) fn begin_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
    ) -> u64 {
        self.next_looped_metadata_intent_id =
            self.next_looped_metadata_intent_id.saturating_add(1).max(1);
        let intent_id = self.next_looped_metadata_intent_id;
        self.looped_metadata_intents
            .insert((source_id.clone(), relative_path.to_path_buf()), intent_id);
        intent_id
    }

    /// Return whether a Loop/One-shot completion still owns the current optimistic edit.
    pub(crate) fn looped_metadata_intent_matches(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) -> bool {
        self.looped_metadata_intents
            .get(&(source_id.clone(), relative_path.to_path_buf()))
            .copied()
            == Some(intent_id)
    }

    /// Clear a completed Loop/One-shot intent only if it is still the current owner.
    pub(crate) fn finish_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) {
        let key = (source_id.clone(), relative_path.to_path_buf());
        if self.looped_metadata_intents.get(&key).copied() == Some(intent_id) {
            self.looped_metadata_intents.remove(&key);
        }
    }

    /// Follow a successful rename so stale metadata completions check the live path.
    pub(crate) fn remap_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        old_relative: &std::path::Path,
        new_relative: &std::path::Path,
    ) {
        if old_relative == new_relative {
            return;
        }
        let old_key = (source_id.clone(), old_relative.to_path_buf());
        if let Some(intent_id) = self.looped_metadata_intents.remove(&old_key) {
            self.looped_metadata_intents
                .insert((source_id.clone(), new_relative.to_path_buf()), intent_id);
        }
    }
}

/// Decision for browser rename input received while the generic file-op lane is busy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrowserRenameBusyDecision {
    /// The same rename target is already active or already queued; keep one coherent operation.
    Collapsed,
    /// A materially different auto-rename request was retained for one follow-up pass.
    Queued,
    /// The active file op was not started by browser rename dispatch.
    UnrelatedFileOp,
}

/// Stable browser rename intent key scoped by source and requested old/new paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserRenameIntentKey {
    pub(crate) source_id: SourceId,
    pub(crate) targets: Vec<(PathBuf, PathBuf)>,
}

impl BrowserRenameIntentKey {
    pub(crate) fn new(source_id: SourceId, mut targets: Vec<(PathBuf, PathBuf)>) -> Self {
        targets.sort();
        targets.dedup();
        Self { source_id, targets }
    }
}

/// One deferred auto-rename request to replay after active browser rename work settles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingBrowserAutoRenameIntent {
    pub(crate) key: BrowserRenameIntentKey,
    pub(crate) source_id: SourceId,
    pub(crate) paths: Vec<PathBuf>,
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
        /// Optimistic edit owner that must still be current before rollback applies.
        intent_id: u64,
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
    /// Restore one normal tag assignment state if the optimistic value is still current.
    NormalTag {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Normalized tag identity.
        normalized_text: String,
        /// Display label to restore when the tag was present before the mutation.
        display_label: String,
        /// Whether the assignment existed before the optimistic mutation.
        before_present: bool,
        /// Value written optimistically before persistence completed.
        expected_present: bool,
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
