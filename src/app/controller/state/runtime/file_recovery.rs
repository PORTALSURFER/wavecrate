//! Runtime state for retained-delete recovery and active recovery operations.

use crate::app::controller::jobs;

/// Runtime state for staged delete recovery flows.
#[derive(Clone, Debug, Default)]
pub(crate) struct FileRecoveryRuntimeState {
    /// Tracks whether staged delete recovery has been scheduled for this session.
    pub(crate) delete_recovery_started: bool,
    /// Explicit retained-delete resolution currently running through the file-op lane.
    pub(crate) active_retained_delete_resolution: Option<jobs::ActiveRetainedDeleteResolution>,
}
