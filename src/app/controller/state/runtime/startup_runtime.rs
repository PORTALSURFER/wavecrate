//! Startup-deferred runtime state.

use super::deferred::DeferredStartupAudioRefreshState;
use crate::app::controller::jobs;

/// Runtime state for startup-deferred background work.
#[derive(Clone, Debug, Default)]
pub(crate) struct StartupRuntimeState {
    /// Startup-deferred source DB maintenance jobs waiting for background launch.
    pub(crate) deferred_source_db_maintenance_jobs: Vec<jobs::SourceDbMaintenanceJob>,
    /// True when deferred startup source DB maintenance should start after first paint.
    pub(crate) deferred_source_db_maintenance_armed: bool,
    /// Number of prepared frame passes since startup configuration was applied.
    pub(crate) frame_prepare_count: u32,
    /// Startup audio refresh deferred until after the first presented frame.
    pub(crate) deferred_audio_refresh: DeferredStartupAudioRefreshState,
}

#[cfg(test)]
mod tests {
    use super::StartupRuntimeState;

    #[test]
    /// Startup runtime defaults to no armed deferred startup work.
    fn default_startup_runtime_is_idle() {
        let state = StartupRuntimeState::default();
        assert!(state.deferred_source_db_maintenance_jobs.is_empty());
        assert!(!state.deferred_source_db_maintenance_armed);
        assert_eq!(state.frame_prepare_count, 0);
        assert!(!state.deferred_audio_refresh.armed);
    }
}
