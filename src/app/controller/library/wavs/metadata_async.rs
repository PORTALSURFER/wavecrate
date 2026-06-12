//! Async metadata persistence helpers that keep controller interactions off the GUI thread.

use super::*;
use crate::app::controller::jobs::{
    AnalysisMetadataMutationOp, JobMessage, MetadataMutationJob, MetadataMutationResult,
    SourceMetadataMutationOp,
};
use crate::app::controller::state::runtime::{MetadataRollback, PendingMetadataMutation};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod analysis_ops;
mod source_ops;
mod tracking;

const SELECTED_SOURCE_MUTATION_CLAIM_GRACE: Duration = Duration::from_millis(750);
const SELECTED_SOURCE_MUTATION_AUTO_SYNC_GRACE: Duration = Duration::from_secs(5);
const METADATA_FILE_OP_PRIORITY_WAIT_TIMEOUT: Duration = Duration::from_secs(8);
const METADATA_FILE_OP_PRIORITY_WAIT_DELAY: Duration = Duration::from_millis(100);

impl AppController {
    /// Queue one metadata mutation batch on a worker thread after optimistic UI updates.
    pub(crate) fn queue_metadata_mutation(
        &mut self,
        source: &SampleSource,
        source_ops: Vec<SourceMetadataMutationOp>,
        analysis_ops: Vec<AnalysisMetadataMutationOp>,
        rollback: Vec<MetadataRollback>,
        refresh_browser_projection: bool,
    ) {
        if source_ops.is_empty() && analysis_ops.is_empty() {
            return;
        }
        let paths = metadata_mutation_paths(&source_ops, &analysis_ops);
        #[cfg(test)]
        let queue_depth_before = self.runtime.source_lane.mutations.pending_metadata_count();
        if cfg!(test) {
            let request_id = self.runtime.jobs.next_metadata_request_id();
            let blocks_file_mutation = !source_ops.is_empty();
            let result = run_metadata_mutation_job(MetadataMutationJob {
                request_id,
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                paths: paths.clone(),
                source_ops,
                analysis_ops,
            });
            self.runtime
                .source_lane
                .mutations
                .insert_metadata_mutation(PendingMetadataMutation {
                    request_id,
                    source_id: source.id.clone(),
                    paths: result.paths.clone(),
                    blocks_file_mutation,
                    rollback,
                    refresh_browser_projection,
                });
            #[cfg(test)]
            crate::app::controller::batch_latency::record(
                crate::app::controller::batch_latency::BatchLatencySample::new(
                    crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue,
                    result.paths.len(),
                    Duration::ZERO,
                )
                .with_detail_count(result.paths.len())
                .with_queue_depths(
                    queue_depth_before,
                    self.runtime.source_lane.mutations.pending_metadata_count(),
                ),
            );
            self.extend_selected_source_mutation_claim_grace(&source.id);
            self.runtime.analysis.pause_claiming();
            self.handle_metadata_mutation_finished_message(result);
            return;
        }
        let request_id = self.runtime.jobs.next_metadata_request_id();
        let blocks_file_mutation = !source_ops.is_empty();
        #[cfg(test)]
        let path_count = paths.len();
        self.runtime
            .source_lane
            .mutations
            .insert_metadata_mutation(PendingMetadataMutation {
                request_id,
                source_id: source.id.clone(),
                paths: paths.clone(),
                blocks_file_mutation,
                rollback,
                refresh_browser_projection,
            });
        #[cfg(test)]
        crate::app::controller::batch_latency::record(
            crate::app::controller::batch_latency::BatchLatencySample::new(
                crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue,
                path_count,
                Duration::ZERO,
            )
            .with_detail_count(path_count)
            .with_queue_depths(
                queue_depth_before,
                self.runtime.source_lane.mutations.pending_metadata_count(),
            ),
        );
        self.extend_selected_source_mutation_claim_grace(&source.id);
        self.runtime.analysis.pause_claiming();
        let job = MetadataMutationJob {
            request_id,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            paths,
            source_ops,
            analysis_ops,
        };
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || run_metadata_mutation_job(job),
            JobMessage::MetadataMutationFinished,
        );
    }

    /// Return whether any streamed file operation is currently in progress.
    pub(crate) fn file_ops_in_progress_for_projection(&self) -> bool {
        self.runtime.jobs.file_ops_in_progress()
    }

    /// Return whether a waveform image render is still in flight.
    pub(crate) fn waveform_render_in_progress_for_projection(&self) -> bool {
        self.runtime.waveform.pending_render.is_some()
    }
}

fn metadata_mutation_paths(
    source_ops: &[SourceMetadataMutationOp],
    analysis_ops: &[AnalysisMetadataMutationOp],
) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    for op in source_ops {
        match op {
            SourceMetadataMutationOp::SetTagAndLocked { relative_path, .. }
            | SourceMetadataMutationOp::SetLooped { relative_path, .. }
            | SourceMetadataMutationOp::SetSoundType { relative_path, .. }
            | SourceMetadataMutationOp::SetUserTag { relative_path, .. }
            | SourceMetadataMutationOp::SetTagNamed { relative_path, .. }
            | SourceMetadataMutationOp::AssignNormalTag { relative_path, .. }
            | SourceMetadataMutationOp::RemoveNormalTag { relative_path, .. }
            | SourceMetadataMutationOp::SetLastPlayedAt { relative_path, .. } => {
                paths.insert(relative_path.clone());
            }
        }
    }
    for op in analysis_ops {
        match op {
            AnalysisMetadataMutationOp::SetBpm { relative_path, .. }
            | AnalysisMetadataMutationOp::SetLoadedDuration { relative_path, .. } => {
                paths.insert(relative_path.clone());
            }
        }
    }
    paths
}

/// Execute one source-scoped metadata mutation batch.
pub(crate) fn run_metadata_mutation_job(job: MetadataMutationJob) -> MetadataMutationResult {
    let started_at = Instant::now();
    let mut result = Ok(());
    wait_for_file_op_priority_to_clear(&job);
    if !job.source_ops.is_empty() {
        result = source_ops::run_source_metadata_ops(&job);
    }
    if result.is_ok() && !job.analysis_ops.is_empty() {
        result = analysis_ops::run_analysis_metadata_ops(&job);
    }
    MetadataMutationResult {
        request_id: job.request_id,
        source_id: job.source_id,
        paths: job.paths,
        elapsed: started_at.elapsed(),
        result,
    }
}

/// Give source-local file operations first chance at their DB rewrite lane.
fn wait_for_file_op_priority_to_clear(job: &MetadataMutationJob) {
    let deadline = Instant::now() + METADATA_FILE_OP_PRIORITY_WAIT_TIMEOUT;
    while crate::app::controller::library::source_write_priority::file_op_write_priority_active(
        &job.source_id,
    ) {
        if Instant::now() >= deadline {
            tracing::warn!(
                source_id = %job.source_id,
                request_id = job.request_id,
                waited_ms = METADATA_FILE_OP_PRIORITY_WAIT_TIMEOUT.as_millis(),
                "metadata mutation waited for active source file-op priority"
            );
            return;
        }
        std::thread::sleep(METADATA_FILE_OP_PRIORITY_WAIT_DELAY);
    }
}

#[cfg(test)]
mod tests;
