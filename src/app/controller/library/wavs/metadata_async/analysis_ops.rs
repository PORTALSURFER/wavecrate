use crate::app::controller::jobs::{AnalysisMetadataMutationOp, MetadataMutationJob};
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceDatabase;
use std::path::{Path, PathBuf};

pub(super) fn run_analysis_metadata_ops(job: &MetadataMutationJob) -> Result<(), String> {
    let mut conn = analysis_jobs::open_source_db(&job.source_root)?;
    let source_db = SourceDatabase::open_for_ui_read(&job.source_root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let duration_updates = collect_loaded_duration_updates(job, &source_db)?;
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
            let resolved_path = resolve_stale_browser_rename_path(job, &source_db, relative_path)?;
            Ok(analysis_jobs::build_sample_id(
                job.source_id.as_str(),
                &resolved_path,
            ))
        })
        .collect::<Result<_, String>>()?;
    let tx = analysis_jobs::db::telemetry::begin_immediate_transaction(
        &mut conn,
        "analysis_metadata_mutation",
    )
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
    analysis_jobs::db::telemetry::commit_transaction(tx, "analysis_metadata_mutation")
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
    source_db: &SourceDatabase,
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
            let resolved_path = resolve_stale_browser_rename_path(job, source_db, relative_path)?;
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
    db: &SourceDatabase,
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
    if db
        .entry_for_path(&new_relative)
        .map_err(|err| format!("Failed to resolve renamed metadata target: {err}"))?
        .is_some()
    {
        return Ok(new_relative);
    }
    Ok(relative_path.to_path_buf())
}
