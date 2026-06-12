//! Runtime state for deferred configuration persistence.

use std::time::Instant;

/// Live volume/config persistence debounce and active request state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ConfigPersistenceRuntimeState {
    /// True when a live volume change is pending persistence.
    pub(crate) volume_persist_dirty: bool,
    /// Debounce deadline for committing a pending volume write.
    pub(crate) volume_persist_deadline: Option<Instant>,
    /// Last persisted volume in milli-units (`0..=1000`).
    pub(crate) last_persisted_volume_milli: Option<u16>,
    /// Active deferred volume/config persistence request, when any.
    pub(crate) pending_config_persist: Option<PendingConfigPersist>,
}

/// Active deferred configuration persistence request.
#[derive(Clone, Debug)]
pub(crate) struct PendingConfigPersist {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Last queued normalized volume value.
    pub(crate) volume: f32,
    /// Time when the request was queued.
    pub(crate) queued_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::ConfigPersistenceRuntimeState;

    #[test]
    /// Default config persistence runtime starts with no dirty or active write state.
    fn default_config_persistence_runtime_is_idle() {
        let state = ConfigPersistenceRuntimeState::default();
        assert!(!state.volume_persist_dirty);
        assert!(state.volume_persist_deadline.is_none());
        assert!(state.last_persisted_volume_milli.is_none());
        assert!(state.pending_config_persist.is_none());
    }
}
