//! Async metadata persistence helpers that keep controller interactions off the GUI thread.

use super::*;
use crate::app::controller::jobs::{
    AnalysisMetadataMutationOp, JobMessage, MetadataMutationJob, MetadataMutationResult,
    SourceMetadataMutationOp,
};
use crate::app::controller::state::runtime::{MetadataRollback, PendingMetadataMutation};
use crate::sample_sources::SourceDatabase;
use rusqlite::TransactionBehavior;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const SELECTED_SOURCE_MUTATION_CLAIM_GRACE: Duration = Duration::from_millis(750);
const SELECTED_SOURCE_MUTATION_AUTO_SYNC_GRACE: Duration = Duration::from_secs(5);

impl AppController {
    pub(crate) fn selected_source_claim_pause_grace_active(&mut self, now: Instant) -> bool {
        let Some(source_id) = self.selected_source_id() else {
            return false;
        };
        self.runtime
            .source_lane
            .mutations
            .claim_pause_grace_active(&source_id, now)
    }

    pub(crate) fn extend_selected_source_mutation_claim_grace(&mut self, source_id: &SourceId) {
        if self.selected_source_id().as_ref() != Some(source_id) {
            return;
        }
        self.runtime.source_lane.mutations.extend_claim_pause_grace(
            source_id,
            Instant::now() + SELECTED_SOURCE_MUTATION_CLAIM_GRACE,
        );
    }

    pub(crate) fn selected_source_auto_sync_grace_active(&mut self, now: Instant) -> bool {
        let Some(source_id) = self.selected_source_id() else {
            return false;
        };
        self.runtime
            .source_lane
            .mutations
            .auto_sync_grace_active(&source_id, now)
    }

    pub(crate) fn source_auto_sync_grace_active(
        &mut self,
        source_id: &SourceId,
        now: Instant,
    ) -> bool {
        self.runtime
            .source_lane
            .mutations
            .auto_sync_grace_active(source_id, now)
    }

    pub(crate) fn extend_selected_source_mutation_auto_sync_grace(
        &mut self,
        source_id: &SourceId,
    ) {
        if self.selected_source_id().as_ref() != Some(source_id) {
            return;
        }
        self.runtime.source_lane.mutations.extend_auto_sync_grace(
            source_id,
            Instant::now() + SELECTED_SOURCE_MUTATION_AUTO_SYNC_GRACE,
        );
    }

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
            self.extend_selected_source_mutation_claim_grace(&source.id);
            self.runtime.analysis.pause_claiming();
            self.handle_metadata_mutation_finished_message(result);
            return;
        }
        let request_id = self.runtime.jobs.next_metadata_request_id();
        let blocks_file_mutation = !source_ops.is_empty();
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

    /// Return whether one sample path already has an optimistic metadata write in flight.
    pub(crate) fn metadata_mutation_pending_for(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> bool {
        self.runtime
            .source_lane
            .mutations
            .metadata_path_pending(source_id, relative_path)
    }

    /// Return whether the currently selected source still has optimistic metadata writes pending.
    pub(crate) fn selected_source_has_pending_metadata_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .source_lane
                .mutations
                .source_has_pending_metadata(&source_id)
        })
    }

    /// Return whether the currently selected source still owns a background file mutation.
    pub(crate) fn selected_source_has_pending_file_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .source_lane
                .mutations
                .source_has_pending_file_mutations(&source_id)
        })
    }

    /// Return whether any streamed file operation is currently in progress.
    pub(crate) fn file_ops_in_progress_for_projection(&self) -> bool {
        self.runtime.jobs.file_ops_in_progress()
    }

    /// Return whether a waveform image render is still in flight.
    pub(crate) fn waveform_render_in_progress_for_projection(&self) -> bool {
        self.runtime.pending_waveform_render.is_some()
    }

    /// Return whether one source currently owns a background file mutation.
    pub(crate) fn source_has_pending_file_mutations(&self, source_id: &SourceId) -> bool {
        self.runtime
            .source_lane
            .mutations
            .source_has_pending_file_mutations(source_id)
    }

    /// Mark one source/path batch as owned by a background file mutation.
    pub(crate) fn begin_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        self.runtime
            .source_lane
            .mutations
            .begin_file_mutation(source_id, paths);
        self.extend_selected_source_mutation_claim_grace(source_id);
        self.extend_selected_source_mutation_auto_sync_grace(source_id);
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        self.runtime
            .source_lane
            .mutations
            .finish_file_mutation(source_id, paths);
        self.extend_selected_source_mutation_claim_grace(source_id);
        self.extend_selected_source_mutation_auto_sync_grace(source_id);
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
    if !job.source_ops.is_empty() {
        result = run_source_metadata_ops(&job.source_root, &job.source_ops);
    }
    if result.is_ok() && !job.analysis_ops.is_empty() {
        result = run_analysis_metadata_ops(&job);
    }
    MetadataMutationResult {
        request_id: job.request_id,
        source_id: job.source_id,
        paths: job.paths,
        elapsed: started_at.elapsed(),
        result,
    }
}

fn run_source_metadata_ops(
    source_root: &Path,
    ops: &[SourceMetadataMutationOp],
) -> Result<(), String> {
    let db = SourceDatabase::open(source_root).map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    for op in ops {
        match op {
            SourceMetadataMutationOp::SetTagAndLocked {
                relative_path,
                tag,
                locked,
            } => {
                batch
                    .set_tag(relative_path, *tag)
                    .map_err(|err| err.to_string())?;
                batch
                    .set_locked(relative_path, *locked)
                    .map_err(|err| err.to_string())?;
            }
            SourceMetadataMutationOp::SetLooped {
                relative_path,
                looped,
            } => {
                batch
                    .set_looped(relative_path, *looped)
                    .map_err(|err| err.to_string())?;
            }
            SourceMetadataMutationOp::SetSoundType {
                relative_path,
                sound_type,
            } => {
                batch
                    .set_sound_type(relative_path, *sound_type)
                    .map_err(|err| err.to_string())?;
            }
            SourceMetadataMutationOp::SetUserTag {
                relative_path,
                user_tag,
            } => {
                batch
                    .set_user_tag(relative_path, user_tag.as_deref())
                    .map_err(|err| err.to_string())?;
            }
            SourceMetadataMutationOp::SetLastPlayedAt {
                relative_path,
                played_at,
            } => {
                batch
                    .set_last_played_at(relative_path, *played_at)
                    .map_err(|err| err.to_string())?;
            }
        }
    }
    batch.commit().map_err(|err| err.to_string())
}

fn run_analysis_metadata_ops(job: &MetadataMutationJob) -> Result<(), String> {
    let mut conn = analysis_jobs::open_source_db(&job.source_root)?;
    let duration_updates = collect_loaded_duration_updates(job)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Failed to start analysis metadata transaction: {err}"))?;
    let bpm_ops: Vec<_> = job
        .analysis_ops
        .iter()
        .filter_map(|op| match op {
            AnalysisMetadataMutationOp::SetBpm { relative_path, bpm } => Some((relative_path, bpm)),
            AnalysisMetadataMutationOp::SetLoadedDuration { .. } => None,
        })
        .collect();
    if !bpm_ops.is_empty() {
        let sample_ids: Vec<String> = bpm_ops
            .iter()
            .map(|(relative_path, _)| {
                analysis_jobs::build_sample_id(job.source_id.as_str(), relative_path)
            })
            .collect();
        let bpm = bpm_ops.first().and_then(|(_, bpm)| **bpm);
        analysis_jobs::db::update_sample_bpms_in_tx(&tx, &sample_ids, bpm.map(f64::from))?;
    }
    for update in &duration_updates {
        analysis_jobs::db::upsert_samples_in_tx(
            &tx,
            std::slice::from_ref(&update.sample_metadata),
        )?;
        analysis_jobs::update_sample_duration(
            &tx,
            &update.sample_metadata.sample_id,
            update.duration_seconds,
            update.sample_rate,
        )?;
        if let Some(long_sample_mark) = update.long_sample_mark {
            analysis_jobs::update_sample_long_mark(
                &tx,
                &update.sample_metadata.sample_id,
                long_sample_mark,
            )?;
        }
    }
    tx.commit()
        .map_err(|err| format!("Failed to commit analysis metadata transaction: {err}"))?;
    Ok(())
}

struct LoadedDurationUpdate {
    sample_metadata: analysis_jobs::SampleMetadata,
    duration_seconds: f32,
    sample_rate: u32,
    long_sample_mark: Option<bool>,
}

fn collect_loaded_duration_updates(
    job: &MetadataMutationJob,
) -> Result<Vec<LoadedDurationUpdate>, String> {
    let mut updates = Vec::new();
    for op in &job.analysis_ops {
        if let AnalysisMetadataMutationOp::SetLoadedDuration {
            relative_path,
            duration_seconds,
            sample_rate,
            long_sample_mark,
        } = op
        {
            let absolute = job.source_root.join(relative_path);
            let (file_size, modified_ns) =
                crate::app::controller::library::wav_io::file_metadata(&absolute)?;
            let sample_id = analysis_jobs::build_sample_id(job.source_id.as_str(), relative_path);
            let content_hash = analysis_jobs::fast_content_hash(file_size, modified_ns);
            updates.push(LoadedDurationUpdate {
                sample_metadata: analysis_jobs::SampleMetadata {
                    sample_id,
                    content_hash,
                    size: file_size,
                    mtime_ns: modified_ns,
                },
                duration_seconds: *duration_seconds,
                sample_rate: *sample_rate,
                long_sample_mark: *long_sample_mark,
            });
        }
    }
    Ok(updates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_mutation_paths_dedup_across_source_and_analysis_ops() {
        let paths = metadata_mutation_paths(
            &[
                SourceMetadataMutationOp::SetLooped {
                    relative_path: PathBuf::from("one.wav"),
                    looped: true,
                },
                SourceMetadataMutationOp::SetLastPlayedAt {
                    relative_path: PathBuf::from("two.wav"),
                    played_at: 5,
                },
            ],
            &[
                AnalysisMetadataMutationOp::SetBpm {
                    relative_path: PathBuf::from("one.wav"),
                    bpm: Some(120.0),
                },
                AnalysisMetadataMutationOp::SetLoadedDuration {
                    relative_path: PathBuf::from("two.wav"),
                    duration_seconds: 1.0,
                    sample_rate: 44_100,
                    long_sample_mark: Some(false),
                },
            ],
        );

        assert_eq!(
            paths.into_iter().collect::<Vec<_>>(),
            vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
        );
    }
}
