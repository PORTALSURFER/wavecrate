//! Runtime state for browser selection, metadata deltas, and deferred browser work.

use super::deferred::{
    BrowserSelectionTransition, PendingBrowserFeatureCacheRefresh, PendingLoadedDurationMetadata,
};
use crate::app::controller::state::audio::PendingAgeUpdate;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

/// Browser-facing runtime state and deferred metadata work.
#[derive(Clone, Default)]
pub(crate) struct BrowserRuntimeState {
    /// Pending playback-age DB update moved out of input action handlers.
    pub(crate) pending_age_update_commit: Option<PendingAgeUpdate>,
    /// Earliest frame time when deferred playback-age persistence may run.
    pub(crate) pending_age_update_commit_not_before: Option<Instant>,
    /// Browser-selection candidate lifecycle spanning preview, commit, loading, and handoff.
    pub(crate) selection_transition: Option<BrowserSelectionTransition>,
    /// Active async browser feature-cache refresh awaiting apply.
    pub(crate) pending_feature_cache_refresh: Option<PendingBrowserFeatureCacheRefresh>,
    /// Pending duration/long-mark metadata write moved out of waveform load hot path.
    pub(crate) pending_loaded_duration_metadata: Option<PendingLoadedDurationMetadata>,
    /// Earliest frame time when deferred duration metadata persistence may run.
    pub(crate) pending_loaded_duration_metadata_not_before: Option<Instant>,
    /// Source-relative metadata paths that must ride the next async browser-search job.
    pub(crate) pending_search_metadata_delta_paths: BTreeSet<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::BrowserRuntimeState;

    #[test]
    /// Default browser runtime should have no pending deferred browser work.
    fn default_browser_runtime_is_idle() {
        let state = BrowserRuntimeState::default();
        assert!(state.pending_age_update_commit.is_none());
        assert!(state.selection_transition.is_none());
        assert!(state.pending_feature_cache_refresh.is_none());
        assert!(state.pending_loaded_duration_metadata.is_none());
        assert!(state.pending_search_metadata_delta_paths.is_empty());
    }
}
