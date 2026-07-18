//! Runtime state for async similarity queries.

use super::deferred::{
    LoadedSimilarityQueryCache, PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh,
    PendingLoadedSimilarityQuery, PendingSimilarityFilterRebuild,
};
use std::time::Instant;

/// Runtime state for similarity query handoff.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimilarityRuntimeState {
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

#[cfg(test)]
mod tests {
    use super::SimilarityRuntimeState;

    #[test]
    /// Default similarity runtime should have no async query ownership.
    fn default_similarity_runtime_is_idle() {
        let state = SimilarityRuntimeState::default();
        assert!(state.pending_refresh.is_none());
        assert!(state.pending_focused_query.is_none());
        assert!(state.pending_loaded_query.is_none());
        assert!(state.loaded_query_cache.is_none());
        assert!(state.pending_filter_rebuild.is_none());
    }
}
