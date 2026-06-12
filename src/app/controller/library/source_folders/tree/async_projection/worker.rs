//! Worker-side folder projection execution and telemetry.

use super::projection_telemetry;
use super::snapshot::{build_refresh_projection_snapshot, build_reprojected_snapshot};
use crate::app::controller::jobs::{
    FolderProjectionJob, FolderProjectionResult, FolderProjectionWork,
};
use std::time::Instant;

/// Run folder projection work and return the request-tagged result.
pub(super) fn run_folder_projection(job: FolderProjectionJob) -> FolderProjectionResult {
    let start = Instant::now();
    let snapshot = match job.work {
        FolderProjectionWork::RefreshAvailable {
            source_root,
            loaded_relative_paths,
            disk_folders,
            cached_available,
            cached_available_show_all_folders,
            pending_wav_load,
        } => build_refresh_projection_snapshot(
            job.model,
            &source_root,
            loaded_relative_paths,
            disk_folders,
            cached_available,
            cached_available_show_all_folders,
            pending_wav_load,
            job.has_source,
        ),
        FolderProjectionWork::Reproject { snapshot } => {
            build_reprojected_snapshot(job.model, snapshot, job.has_source)
        }
    };
    let elapsed = start.elapsed();
    projection_telemetry::record_folder_projection_worker(
        elapsed,
        snapshot.tree.available.len(),
        snapshot.view.rows.len(),
    );
    FolderProjectionResult {
        request_id: job.request_id,
        pane: job.pane,
        source_id: job.source_id,
        elapsed,
        snapshot,
    }
}
