//! Runtime state for source scan and watcher synchronization.

use crate::sample_sources::SourceId;
use std::collections::HashMap;
use std::time::Instant;

/// Per-source debounce state for automatic quick and targeted scans.
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceSyncRuntimeState {
    pub(crate) auto_sync_last_by_source: HashMap<SourceId, Instant>,
}

#[cfg(test)]
mod tests {
    use super::SourceSyncRuntimeState;

    #[test]
    fn default_source_sync_runtime_has_no_sync_history() {
        let state = SourceSyncRuntimeState::default();
        assert!(state.auto_sync_last_by_source.is_empty());
    }
}
