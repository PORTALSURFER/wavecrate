//! Async metadata persistence helpers that keep controller interactions off the GUI thread.

use super::*;
use crate::app::controller::jobs::{
    AnalysisMetadataMutationOp, JobMessage, MetadataMutationJob, MetadataMutationResult,
    SourceMetadataMutationOp,
};
use crate::app::controller::state::runtime::{MetadataRollback, PendingMetadataMutation};
use crate::sample_sources::SourceDatabase;
use std::collections::BTreeSet;
use std::time::Instant;

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
        if cfg!(test) {
            let request_id = self.runtime.jobs.next_metadata_request_id();
            let result = run_metadata_mutation_job(MetadataMutationJob {
                request_id,
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                paths: paths.clone(),
                source_ops,
                analysis_ops,
            });
            self.runtime.pending_metadata_mutations.insert(
                request_id,
                PendingMetadataMutation {
                    request_id,
                    source_id: source.id.clone(),
                    paths: result.paths.clone(),
                    rollback,
                    refresh_browser_projection,
                },
            );
            for path in &result.paths {
                self.runtime
                    .pending_metadata_paths
                    .insert((source.id.clone(), path.clone()));
            }
            self.handle_metadata_mutation_finished_message(result);
            return;
        }
        let request_id = self.runtime.jobs.next_metadata_request_id();
        for path in &paths {
            self.runtime
                .pending_metadata_paths
                .insert((source.id.clone(), path.clone()));
        }
        self.runtime.pending_metadata_mutations.insert(
            request_id,
            PendingMetadataMutation {
                request_id,
                source_id: source.id.clone(),
                paths: paths.clone(),
                rollback,
                refresh_browser_projection,
            },
        );
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
            .pending_metadata_paths
            .contains(&(source_id.clone(), relative_path.to_path_buf()))
    }

    /// Return whether the currently selected source still has optimistic metadata writes pending.
    pub(crate) fn selected_source_has_pending_metadata_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .pending_metadata_paths
                .iter()
                .any(|(pending_source_id, _)| pending_source_id == &source_id)
        })
    }

    /// Return whether the currently selected source still owns a background file mutation.
    pub(crate) fn selected_source_has_pending_file_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .pending_file_mutation_sources
                .contains(&source_id)
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
            .pending_file_mutation_sources
            .contains(source_id)
    }

    /// Mark one source/path batch as owned by a background file mutation.
    pub(crate) fn begin_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        self.runtime
            .pending_file_mutation_sources
            .insert(source_id.clone());
        for path in paths {
            self.runtime
                .pending_file_mutation_paths
                .insert((source_id.clone(), path));
        }
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        for path in paths {
            self.runtime
                .pending_file_mutation_paths
                .remove(&(source_id.clone(), path));
        }
        let still_has_paths = self
            .runtime
            .pending_file_mutation_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id);
        if !still_has_paths {
            self.runtime.pending_file_mutation_sources.remove(source_id);
        }
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
        analysis_jobs::update_sample_bpms(&mut conn, &sample_ids, bpm)?;
    }
    for op in &job.analysis_ops {
        if let AnalysisMetadataMutationOp::SetLoadedDuration {
            relative_path,
            duration_seconds,
            sample_rate,
            long_sample_mark,
        } = op
        {
            let source = SampleSource {
                id: job.source_id.clone(),
                root: job.source_root.clone(),
            };
            let (file_size, modified_ns) = crate::app::controller::library::wav_io::file_metadata(
                &source.root.join(relative_path),
            )?;
            let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
            let content_hash = analysis_jobs::fast_content_hash(file_size, modified_ns);
            analysis_jobs::upsert_samples(
                &mut conn,
                &[analysis_jobs::SampleMetadata {
                    sample_id: sample_id.clone(),
                    content_hash,
                    size: file_size,
                    mtime_ns: modified_ns,
                }],
            )?;
            analysis_jobs::update_sample_duration(
                &conn,
                &sample_id,
                *duration_seconds,
                *sample_rate,
            )?;
            if let Some(long_sample_mark) = long_sample_mark {
                analysis_jobs::update_sample_long_mark(&conn, &sample_id, *long_sample_mark)?;
            }
        }
    }
    Ok(())
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
