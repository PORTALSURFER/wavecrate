use crate::app::controller::jobs::{SampleAutoRenameProgress, SourceHydrationKind};
use crate::app::state::FolderPaneId;
use crate::sample_sources::Rating;
use crate::sample_sources::SourceId;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

mod auto_rename_batch;
mod folder_projection;
mod hydration;
mod mutations;

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

/// Runtime tracking for pane-scoped folder projection requests.
#[derive(Clone, Debug, Default)]
pub(crate) struct FolderProjectionRuntime {
    /// Pending pane-scoped folder projection jobs keyed by owning sidebar pane.
    pending: HashMap<FolderPaneId, PendingFolderProjection>,
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
    /// Active source-scoped browser auto-rename batch row state.
    active_auto_rename_batch: Option<ActiveAutoRenameBatchState>,
}

/// UI row state for one requested path in an active auto-rename batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AutoRenameBatchRowState {
    Queued,
    Active,
    Completed,
    Skipped,
    Failed,
}

/// Source-scoped snapshot exposed to controller/UI projection code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveAutoRenameBatchSnapshot {
    pub(crate) source_id: SourceId,
    pub(crate) rows: Vec<AutoRenameBatchRowSnapshot>,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) remaps: Vec<(PathBuf, PathBuf)>,
}

/// Snapshot row for one requested auto-rename target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AutoRenameBatchRowSnapshot {
    pub(crate) requested_path: PathBuf,
    pub(crate) current_path: PathBuf,
    pub(crate) state: AutoRenameBatchRowState,
}

#[derive(Clone, Debug)]
struct ActiveAutoRenameBatchState {
    source_id: SourceId,
    requested_paths: Vec<PathBuf>,
    states: HashMap<PathBuf, AutoRenameBatchRowState>,
    remaps: HashMap<PathBuf, PathBuf>,
    current_requested_path: Option<PathBuf>,
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
