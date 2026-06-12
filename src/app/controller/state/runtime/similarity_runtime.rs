//! Runtime state for similarity preparation and async similarity queries.

use super::deferred::{
    LoadedSimilarityQueryCache, PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh,
    PendingLoadedSimilarityQuery, PendingSimilarityFilterRebuild,
};
use crate::sample_sources::SourceId;
use std::time::Instant;

/// Runtime state for similarity preparation and query handoff.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimilarityRuntimeState {
    pub(crate) prep: Option<SimilarityPrepState>,
    pub(crate) prep_last_error: Option<String>,
    pub(crate) prep_last_attempt: Option<Instant>,
    pub(crate) prep_force_full_analysis_next: bool,
    /// Pending focused-similarity refresh moved out of input action handlers.
    pub(crate) pending_refresh: Option<PendingFocusedSimilarityRefresh>,
    /// Earliest frame time when deferred focused-similarity refresh may run.
    pub(crate) pending_refresh_not_before: Option<Instant>,
    /// Active async focused-similarity highlight computation awaiting apply.
    pub(crate) pending_focused_query: Option<PendingFocusedSimilarityQuery>,
    /// Active async follow-loaded similarity query computation awaiting apply.
    pub(crate) pending_loaded_query: Option<PendingLoadedSimilarityQuery>,
    /// Retained loaded-similarity query cached by source snapshot and anchor sample.
    pub(crate) loaded_query_cache: Option<LoadedSimilarityQueryCache>,
    /// Pending manual similarity-filter rebuild scheduled after destructive wav mutations.
    pub(crate) pending_filter_rebuild: Option<PendingSimilarityFilterRebuild>,
}

#[derive(Clone, Debug)]
pub(crate) struct SimilarityPrepState {
    pub(crate) source_id: SourceId,
    pub(crate) stage: SimilarityPrepStage,
    pub(crate) umap_version: String,
    pub(crate) scan_completed_at: Option<i64>,
    pub(crate) skip_backfill: bool,
    pub(crate) force_full_analysis: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SimilarityPrepStage {
    AwaitScan,
    AwaitEmbeddings,
    Finalizing,
}

#[cfg(test)]
mod tests {
    use super::SimilarityRuntimeState;

    #[test]
    /// Default similarity runtime should have no prep or async query ownership.
    fn default_similarity_runtime_is_idle() {
        let state = SimilarityRuntimeState::default();
        assert!(state.prep.is_none());
        assert!(state.prep_last_error.is_none());
        assert!(state.prep_last_attempt.is_none());
        assert!(!state.prep_force_full_analysis_next);
        assert!(state.pending_refresh.is_none());
        assert!(state.pending_focused_query.is_none());
        assert!(state.pending_loaded_query.is_none());
        assert!(state.loaded_query_cache.is_none());
        assert!(state.pending_filter_rebuild.is_none());
    }
}
