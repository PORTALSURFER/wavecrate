//! Job message and DTO types shared across controller workers.

use super::*;

#[derive(Debug)]
#[cfg_attr(test, allow(dead_code))]
pub(crate) enum JobMessage {
    WavLoaded(WavLoadResult),
    AudioLoaded(AudioLoadResult),
    RecordingWaveformLoaded(RecordingWaveformLoadResult),
    Scan(ScanJobMessage),
    FolderScanFinished(FolderScanResult),
    SourceWatch(SourceWatchEvent),
    TrashMove(trash_move::TrashMoveMessage),
    FolderDeleteRecoveryFinished(DeleteRecoveryReport),
    FileOps(FileOpMessage),
    Analysis(AnalysisJobMessage),
    AnalysisFailuresLoaded(AnalysisFailuresResult),
    UmapBuilt(UmapBuildResult),
    UmapClustersBuilt(UmapClusterBuildResult),
    SimilarityPrepared(SimilarityPrepResult),
    UpdateChecked(UpdateCheckResult),
    IssueGatewayCreated(IssueGatewayCreateResult),
    IssueGatewayAuthed(IssueGatewayAuthResult),
    IssueTokenLoaded(IssueTokenLoadResult),
    IssueTokenSaved(IssueTokenSaveResult),
    IssueTokenDeleted(IssueTokenDeleteResult),
    BrowserSearchFinished(SearchResult),
    SourceDbMaintenanceFinished(SourceDbMaintenanceResult),
    Normalized(NormalizationResult),
}

#[derive(Debug)]
pub(crate) struct SearchJob {
    /// Monotonic request identifier used to discard stale async search results.
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) source_root: PathBuf,
    pub(crate) query: String,
    pub(crate) filter: crate::app::state::TriageFlagFilter,
    /// Rating levels selected for filtering (-3..=3). Empty means no rating filter.
    pub(crate) rating_filter: BTreeSet<i8>,
    pub(crate) sort: crate::app::state::SampleBrowserSort,
    pub(crate) similar_query: Option<crate::app::state::SimilarQuery>,
    pub(crate) folder_selection: Option<BTreeSet<PathBuf>>,
    pub(crate) folder_negated: Option<BTreeSet<PathBuf>>,
    pub(crate) root_mode: crate::app::state::RootFolderFilterMode,
}

#[derive(Debug)]
pub(crate) struct SearchResult {
    /// Request identifier echoed from [`SearchJob::request_id`].
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) query: String,
    pub(crate) visible: crate::app::state::VisibleRows,
    /// Shared triage row indexes tagged as trash.
    pub(crate) trash: Arc<[usize]>,
    /// Shared triage row indexes tagged as neutral.
    pub(crate) neutral: Arc<[usize]>,
    /// Shared triage row indexes tagged as keep.
    pub(crate) keep: Arc<[usize]>,
    /// Shared query score payload aligned to absolute row indexes.
    pub(crate) scores: Arc<[Option<i64>]>,
}

/// Result of a background folder scan for a source root.
#[derive(Debug)]
pub(crate) struct FolderScanResult {
    /// Request identifier for this scan.
    pub(crate) request_id: u64,
    /// Source identifier associated with the scan.
    pub(crate) source_id: SourceId,
    /// Relative folder paths discovered on disk.
    pub(crate) folders: BTreeSet<PathBuf>,
}

/// Request payload for creating a new issue through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayJob {
    /// Bearer token used to authorize issue creation.
    pub(crate) token: String,
    /// Serialized issue request body sent to the gateway API.
    pub(crate) request: crate::issue_gateway::api::CreateIssueRequest,
}

/// Poll request for gateway-issued auth completion by request id.
#[derive(Debug)]
pub(crate) struct IssueGatewayPollJob {
    /// Opaque request identifier returned by the create/auth kickoff flow.
    pub(crate) request_id: String,
}

/// Outcome of an issue-create request sent through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayCreateResult {
    /// API result payload or domain error returned by the gateway client.
    pub(crate) result: Result<
        crate::issue_gateway::api::CreateIssueResponse,
        crate::issue_gateway::api::CreateIssueError,
    >,
}

/// Outcome of polling the gateway for an authenticated issue token.
#[derive(Debug)]
pub(crate) struct IssueGatewayAuthResult {
    /// Auth token when polling succeeds, or the terminal polling error.
    pub(crate) result: Result<String, crate::issue_gateway::api::IssueAuthError>,
}

/// Request to save a GitHub issue token to persistent storage.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveJob {
    /// Token value to persist.
    pub(crate) token: String,
    /// Whether the token modal should reopen after save completion.
    pub(crate) reopen_modal: bool,
}

/// Result from attempting to load a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenLoadResult {
    pub(crate) result: Result<Option<String>, crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to save a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveResult {
    pub(crate) token: String,
    pub(crate) reopen_modal: bool,
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to delete a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenDeleteResult {
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}

#[derive(Debug, Clone)]
pub(crate) struct UmapBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: SourceId,
}

#[derive(Debug)]
pub(crate) struct UmapBuildResult {
    pub(crate) umap_version: String,
    pub(crate) result: Result<(), String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UmapClusterBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: Option<SourceId>,
}

#[derive(Debug)]
pub(crate) struct UmapClusterBuildResult {
    #[allow(dead_code)]
    pub(crate) umap_version: String,
    pub(crate) source_id: Option<SourceId>,
    pub(crate) result: Result<crate::analysis::hdbscan::HdbscanStats, String>,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepOutcome {
    pub(crate) cluster_stats: crate::analysis::hdbscan::HdbscanStats,
    #[allow(dead_code)]
    pub(crate) umap_version: String,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<SimilarityPrepOutcome, String>,
}

#[derive(Debug)]
pub(crate) struct AnalysisFailuresResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<std::collections::HashMap<PathBuf, String>, String>,
}

#[derive(Debug)]
pub(crate) struct NormalizationJob {
    pub(crate) source: crate::sample_sources::SampleSource,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct NormalizationResult {
    pub(crate) source_id: crate::sample_sources::SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) result: Result<(u64, i64, crate::sample_sources::Rating), String>,
}

/// Startup-deferred source DB maintenance request.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceJob {
    /// Source id used for status/error attribution.
    pub(crate) source_id: SourceId,
    /// Root path of the source database.
    pub(crate) source_root: PathBuf,
}

/// Summary for one source DB maintenance attempt.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceOutcome {
    /// Source id associated with this outcome.
    pub(crate) source_id: SourceId,
    /// Source root used for maintenance.
    pub(crate) source_root: PathBuf,
    /// Whether this source was skipped due to unchanged revision/schema token.
    pub(crate) skipped: bool,
    /// Number of orphaned analysis rows removed.
    pub(crate) orphan_rows_removed: usize,
    /// Error when maintenance failed after retry attempts.
    pub(crate) error: Option<String>,
}

/// Batched result for deferred source DB maintenance.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceResult {
    /// Per-source maintenance outcomes.
    pub(crate) outcomes: Vec<SourceDbMaintenanceOutcome>,
}
