//! Runtime state for similarity preparation and async similarity queries.

use super::deferred::{
    LoadedSimilarityQueryCache, PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh,
    PendingLoadedSimilarityQuery, PendingSimilarityFilterRebuild,
};
use crate::sample_sources::SourceId;
use std::collections::HashMap;
use std::time::Instant;

/// Runtime state for similarity preparation and query handoff.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimilarityRuntimeState {
    pub(crate) prep: Option<SimilarityPrepState>,
    pub(crate) prep_last_error: Option<String>,
    pub(crate) prep_last_attempt: Option<Instant>,
    pub(crate) prep_force_full_analysis_next: bool,
    /// Active similarity finalizers by source, retained even when prep UI state is canceled.
    active_finalize_sources: HashMap<SourceId, usize>,
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

impl SimilarityRuntimeState {
    /// Retain source-write ownership for one newly started similarity finalizer.
    pub(crate) fn begin_finalize(&mut self, source_id: &SourceId) {
        *self
            .active_finalize_sources
            .entry(source_id.clone())
            .or_default() += 1;
    }

    /// Release source-write ownership for one completed similarity finalizer.
    pub(crate) fn finish_finalize(&mut self, source_id: &SourceId) {
        let Some(active) = self.active_finalize_sources.get_mut(source_id) else {
            return;
        };
        *active = active.saturating_sub(1);
        if *active == 0 {
            self.active_finalize_sources.remove(source_id);
        }
    }

    /// Return whether a finalizer can still write this source database.
    pub(crate) fn finalize_in_progress_for(&self, source_id: &SourceId) -> bool {
        self.active_finalize_sources.contains_key(source_id)
    }
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
        assert!(state.active_finalize_sources.is_empty());
        assert!(state.pending_refresh.is_none());
        assert!(state.pending_focused_query.is_none());
        assert!(state.pending_loaded_query.is_none());
        assert!(state.loaded_query_cache.is_none());
        assert!(state.pending_filter_rebuild.is_none());
    }

    #[test]
    fn similarity_finalize_ownership_is_reference_counted() {
        let source_id = crate::sample_sources::SourceId::new();
        let mut state = SimilarityRuntimeState::default();

        state.begin_finalize(&source_id);
        state.begin_finalize(&source_id);
        state.finish_finalize(&source_id);
        assert!(state.finalize_in_progress_for(&source_id));

        state.finish_finalize(&source_id);
        assert!(!state.finalize_in_progress_for(&source_id));
    }
}
