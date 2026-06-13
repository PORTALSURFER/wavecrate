//! Source-watcher notifications owned by controller job dispatch.

use super::*;

impl ControllerJobs {
    /// Notify the source watcher when scan state transitions.
    pub(super) fn send_source_watch_scan_state(&self, in_progress: bool) {
        self.source_watcher
            .send(SourceWatchCommand::SetScanInProgress { in_progress });
    }

    /// Notify the source watcher that controller-owned file-op paths are active.
    pub(in super::super::super) fn begin_source_watch_file_op(
        &self,
        source_id: SourceId,
        relative_paths: Vec<PathBuf>,
    ) {
        self.source_watcher
            .send(SourceWatchCommand::BeginControllerFileOp {
                source_id,
                relative_paths,
            });
    }

    /// Notify the source watcher that controller-owned file-op paths are complete.
    pub(in super::super::super) fn finish_source_watch_file_op(
        &self,
        source_id: SourceId,
        relative_paths: Vec<PathBuf>,
    ) {
        self.source_watcher
            .send(SourceWatchCommand::FinishControllerFileOp {
                source_id,
                relative_paths,
            });
    }
}
