//! Similarity, UMAP, and analysis-failure DTOs for controller background work.

use super::*;

/// Request to build or refresh UMAP data for one source snapshot.
#[derive(Debug, Clone)]
pub(crate) struct UmapBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: SourceId,
}

/// Result of one UMAP build attempt.
#[derive(Debug)]
pub(crate) struct UmapBuildResult {
    pub(crate) umap_version: String,
    pub(crate) result: Result<(), String>,
}

/// Request to cluster the current UMAP projection.
#[derive(Debug, Clone)]
pub(crate) struct UmapClusterBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: Option<SourceId>,
}

/// Result of one UMAP clustering pass.
#[derive(Debug)]
pub(crate) struct UmapClusterBuildResult {
    pub(crate) source_id: Option<SourceId>,
    pub(crate) result: Result<wavecrate_analysis::hdbscan::HdbscanStats, String>,
}

/// Final prepared similarity payload applied back to controller state.
#[derive(Debug)]
pub(crate) struct SimilarityPrepOutcome {
    pub(crate) cluster_stats: wavecrate_analysis::hdbscan::HdbscanStats,
}

/// Result of one similarity-preparation run.
#[derive(Debug)]
pub(crate) struct SimilarityPrepResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<SimilarityPrepOutcome, String>,
}

/// Path-based similarity highlight payload computed off the controller thread.
#[derive(Debug)]
pub(crate) struct FocusedSimilarityPaths {
    /// Stable sample identifier for the focused anchor sample.
    pub(crate) sample_id: String,
    /// Candidate relative paths in descending similarity order.
    pub(crate) paths: Vec<PathBuf>,
    /// Similarity scores aligned to [`Self::paths`].
    pub(crate) scores: Vec<f32>,
    /// Focused entry index captured when the request was queued.
    pub(crate) anchor_index: Option<usize>,
}

/// Async result for one focused-similarity highlight refresh request.
#[derive(Debug)]
pub(crate) struct FocusedSimilarityResult {
    /// Monotonic request identifier used to discard stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the focused selection when the request started.
    pub(crate) source_id: SourceId,
    /// Focused relative path expected to still be selected on apply.
    pub(crate) relative_path: PathBuf,
    /// Computed highlight payload or the terminal error.
    pub(crate) result: Result<Option<FocusedSimilarityPaths>, String>,
}

/// Async result for one follow-loaded similarity query build request.
#[derive(Debug)]
pub(crate) struct LoadedSimilarityQueryResult {
    /// Monotonic request identifier used to discard stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the loaded sample when the request started.
    pub(crate) source_id: SourceId,
    /// Loaded relative path expected to still be active on apply.
    pub(crate) relative_path: PathBuf,
    /// Browser snapshot key the built query still aligns with.
    pub(crate) key: crate::app::controller::FeatureCacheKey,
    /// Similarity query payload plus retained source snapshot or the terminal error.
    pub(crate) result:
        Result<crate::app::controller::state::runtime::LoadedSimilarityQueryData, String>,
}

/// Result of loading persisted analysis failures for one source.
#[derive(Debug)]
pub(crate) struct AnalysisFailuresResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<std::collections::HashMap<PathBuf, String>, String>,
}
