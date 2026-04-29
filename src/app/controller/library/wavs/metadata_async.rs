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
const METADATA_FILE_OP_PRIORITY_WAIT_TIMEOUT: Duration = Duration::from_secs(8);
const METADATA_FILE_OP_PRIORITY_WAIT_DELAY: Duration = Duration::from_millis(100);

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

    pub(crate) fn extend_selected_source_mutation_auto_sync_grace(&mut self, source_id: &SourceId) {
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
        let paths = paths.into_iter().collect::<Vec<_>>();
        let source_became_active = self
            .runtime
            .source_lane
            .mutations
            .begin_file_mutation(source_id, paths.clone());
        if source_became_active {
            crate::app::controller::library::source_write_priority::begin_file_op_write_priority(
                source_id,
            );
        }
        self.runtime
            .jobs
            .begin_source_watch_file_op(source_id.clone(), paths);
        self.extend_selected_source_mutation_claim_grace(source_id);
        self.extend_selected_source_mutation_auto_sync_grace(source_id);
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        let paths = paths.into_iter().collect::<Vec<_>>();
        let source_became_inactive = self
            .runtime
            .source_lane
            .mutations
            .finish_file_mutation(source_id, paths.clone());
        if source_became_inactive {
            crate::app::controller::library::source_write_priority::finish_file_op_write_priority(
                source_id,
            );
        }
        self.runtime
            .jobs
            .finish_source_watch_file_op(source_id.clone(), paths);
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
            SourceMetadataMutationOp::AssignNormalTag {
                relative_path,
                label,
            } => {
                batch
                    .assign_tag_to_path(relative_path, label)
                    .map_err(|err| err.to_string())?;
            }
            SourceMetadataMutationOp::RemoveNormalTag {
                relative_path,
                label,
            } => {
                batch
                    .remove_tag_from_path(relative_path, label)
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
    let bpm_ops: Vec<_> = job
        .analysis_ops
        .iter()
        .filter_map(|op| match op {
            AnalysisMetadataMutationOp::SetBpm { relative_path, bpm } => Some((relative_path, bpm)),
            AnalysisMetadataMutationOp::SetLoadedDuration { .. } => None,
        })
        .collect();
    let bpm_sample_ids: Vec<String> = bpm_ops
        .iter()
        .map(|(relative_path, _)| {
            let resolved_path = resolve_stale_browser_rename_path(job, relative_path)?;
            Ok(analysis_jobs::build_sample_id(
                job.source_id.as_str(),
                &resolved_path,
            ))
        })
        .collect::<Result<_, String>>()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Failed to start analysis metadata transaction: {err}"))?;
    if !bpm_ops.is_empty() {
        let bpm = bpm_ops.first().and_then(|(_, bpm)| **bpm);
        analysis_jobs::db::update_sample_bpms_in_tx(&tx, &bpm_sample_ids, bpm.map(f64::from))?;
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
            let resolved_path = resolve_stale_browser_rename_path(job, relative_path)?;
            let absolute = job.source_root.join(&resolved_path);
            let (file_size, modified_ns) =
                crate::app::controller::library::wav_io::file_metadata(&absolute)?;
            let sample_id = analysis_jobs::build_sample_id(job.source_id.as_str(), &resolved_path);
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

fn resolve_stale_browser_rename_path(
    job: &MetadataMutationJob,
    relative_path: &Path,
) -> Result<PathBuf, String> {
    if job.source_root.join(relative_path).exists() {
        return Ok(relative_path.to_path_buf());
    }
    let Some(new_relative) =
        crate::app::controller::library::source_write_priority::completed_browser_rename_target(
            &job.source_id,
            relative_path,
        )
    else {
        return Ok(relative_path.to_path_buf());
    };
    if SourceDatabase::open(&job.source_root)
        .map_err(|err| format!("Database unavailable: {err}"))?
        .entry_for_path(&new_relative)
        .map_err(|err| format!("Failed to resolve renamed metadata target: {err}"))?
        .is_some()
    {
        return Ok(new_relative);
    }
    Ok(relative_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::source_write_priority;

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

    #[test]
    fn metadata_mutation_waits_behind_same_source_file_op_priority() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).expect("create source root");
        let relative_path = PathBuf::from("alpha.wav");
        let db = SourceDatabase::open(&source.root).expect("open source db");
        db.upsert_file(&relative_path, 1, 1)
            .expect("insert source row");
        source_write_priority::begin_file_op_write_priority(&source.id);
        let release_source_id = source.id.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(260));
            source_write_priority::finish_file_op_write_priority(&release_source_id);
        });

        let result = run_metadata_mutation_job(MetadataMutationJob {
            request_id: 7,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            paths: [relative_path.clone()].into_iter().collect(),
            source_ops: vec![SourceMetadataMutationOp::SetUserTag {
                relative_path: relative_path.clone(),
                user_tag: Some(String::from("Vintage")),
            }],
            analysis_ops: Vec::new(),
        });

        assert!(result.elapsed >= Duration::from_millis(200));
        assert!(result.result.is_ok(), "{:?}", result.result);
        assert_eq!(
            db.user_tag_for_path(&relative_path).expect("read user tag"),
            Some(String::from("Vintage"))
        );
    }

    fn persisted_duration_seconds(source: &SampleSource, relative_path: &Path) -> Option<f64> {
        let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
        let conn = analysis_jobs::open_source_db(&source.root).expect("open analysis db");
        conn.query_row(
            "SELECT duration_seconds FROM samples WHERE sample_id = ?1",
            rusqlite::params![sample_id],
            |row| row.get::<_, Option<f64>>(0),
        )
        .ok()
        .flatten()
    }

    #[test]
    fn loaded_duration_metadata_job_follows_completed_browser_rename() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).expect("create source root");
        let old_relative = PathBuf::from("old-name.wav");
        let new_relative = PathBuf::from("new-name.wav");
        let old_absolute = source.root.join(&old_relative);
        let new_absolute = source.root.join(&new_relative);
        std::fs::write(&old_absolute, b"metadata-fixture").expect("write fixture");

        let db = SourceDatabase::open(&source.root).expect("open source db");
        let (old_size, old_modified_ns) =
            crate::app::controller::library::wav_io::file_metadata(&old_absolute)
                .expect("old metadata");
        db.upsert_file(&old_relative, old_size, old_modified_ns)
            .expect("insert old row");
        std::fs::rename(&old_absolute, &new_absolute).expect("rename fixture");
        let (new_size, new_modified_ns) =
            crate::app::controller::library::wav_io::file_metadata(&new_absolute)
                .expect("new metadata");
        let mut batch = db.write_batch().expect("start rename batch");
        batch.remove_file(&old_relative).expect("remove old row");
        batch
            .upsert_file(&new_relative, new_size, new_modified_ns)
            .expect("insert new row");
        batch
            .remap_analysis_sample_identity(&old_relative, &new_relative)
            .expect("remap analysis identity");
        batch.commit().expect("commit rename batch");
        source_write_priority::record_completed_browser_rename(
            &source.id,
            &old_relative,
            &new_relative,
        );

        let result = run_metadata_mutation_job(MetadataMutationJob {
            request_id: 11,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            paths: [old_relative.clone()].into_iter().collect(),
            source_ops: Vec::new(),
            analysis_ops: vec![AnalysisMetadataMutationOp::SetLoadedDuration {
                relative_path: old_relative.clone(),
                duration_seconds: 2.5,
                sample_rate: 44_100,
                long_sample_mark: Some(false),
            }],
        });

        assert!(result.result.is_ok(), "{:?}", result.result);
        assert!(persisted_duration_seconds(&source, &old_relative).is_none());
        assert_eq!(
            persisted_duration_seconds(&source, &new_relative),
            Some(2.5)
        );
    }

    #[test]
    fn loaded_duration_metadata_job_reports_missing_file_without_rename_mapping() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).expect("create source root");
        let relative_path = PathBuf::from("missing.wav");
        let db = SourceDatabase::open(&source.root).expect("open source db");
        db.upsert_file(&relative_path, 1, 1)
            .expect("insert source row");

        let result = run_metadata_mutation_job(MetadataMutationJob {
            request_id: 12,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            paths: [relative_path.clone()].into_iter().collect(),
            source_ops: Vec::new(),
            analysis_ops: vec![AnalysisMetadataMutationOp::SetLoadedDuration {
                relative_path: relative_path.clone(),
                duration_seconds: 1.0,
                sample_rate: 44_100,
                long_sample_mark: None,
            }],
        });

        let err = result.result.expect_err("missing file should still fail");
        assert!(
            err.contains("Failed to read") && err.contains("missing.wav"),
            "expected actionable missing-file error, got: {err}"
        );
    }
}
