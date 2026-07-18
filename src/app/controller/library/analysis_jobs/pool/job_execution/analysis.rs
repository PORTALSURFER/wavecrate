use crate::app::controller::library::analysis_jobs::db;

use super::analysis_cache::{load_existing_embedding, lookup_cache_by_hash};
use super::analysis_db::{
    apply_cached_embedding, apply_cached_features_and_embedding, persist_decoded_analysis_write,
    update_metadata_for_skip,
};
use super::analysis_decode::{DecodeOutcome, decode_for_analysis};
use super::support::JobHeartbeat;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct AnalysisContext<'a> {
    pub(crate) use_cache: bool,
    pub(crate) max_analysis_duration_seconds: f32,
    pub(crate) analysis_sample_rate: u32,
    pub(crate) analysis_version: &'a str,
    pub(crate) cancel: Option<&'a AtomicBool>,
}

fn checkpoint(context: &AnalysisContext<'_>) -> Result<(), String> {
    if context
        .cancel
        .is_some_and(|cancel| cancel.load(Ordering::Acquire))
    {
        Err("Analysis job cancelled".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn run_analysis_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    context: &AnalysisContext<'_>,
) -> Result<(), String> {
    checkpoint(context)?;
    let mut heartbeat = JobHeartbeat::new(std::time::Duration::from_secs(4));
    let job_ids = [job.id];
    heartbeat.touch_jobs(conn, &job_ids)?;
    let content_hash = job
        .content_hash
        .as_deref()
        .ok_or_else(|| format!("Missing content_hash for analysis job {}", job.sample_id))?;
    let current_hash = db::sample_content_hash(conn, &job.sample_id)?;
    if current_hash.as_deref() != Some(content_hash) {
        return Ok(());
    }
    if context.use_cache {
        let cache = lookup_cache_by_hash(conn, content_hash, context.analysis_version)?;
        if let (Some(features), Some(embedding)) = (&cache.features, &cache.embedding) {
            checkpoint(context)?;
            apply_cached_features_and_embedding(
                conn,
                job,
                content_hash,
                features,
                embedding,
                cache.aspect_descriptors.as_ref(),
                context.analysis_version,
            )?;
            return Ok(());
        }
        if let Some(embedding) = cache.embedding.as_ref() {
            checkpoint(context)?;
            apply_cached_embedding(conn, job, embedding)?;
        }
    }

    heartbeat.touch_jobs(conn, &job_ids)?;
    match decode_for_analysis(job, context)? {
        DecodeOutcome::Decoded(decoded) => {
            checkpoint(context)?;
            heartbeat.touch_jobs(conn, &job_ids)?;
            run_analysis_job_with_decoded(conn, job, decoded, context)
        }
        DecodeOutcome::Skipped {
            duration_seconds,
            sample_rate,
        } => {
            checkpoint(context)?;
            heartbeat.touch_jobs(conn, &job_ids)?;
            update_metadata_for_skip(
                conn,
                job,
                duration_seconds,
                sample_rate,
                context.analysis_version,
            )
        }
    }
}

pub(crate) fn run_analysis_job_with_decoded(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    decoded: wavecrate_analysis::AnalysisAudio,
    context: &AnalysisContext<'_>,
) -> Result<(), String> {
    checkpoint(context)?;
    let needs_embedding_upsert = if context.use_cache {
        load_existing_embedding(conn, &job.sample_id)?.is_none()
    } else {
        true
    };
    let write = super::analysis_db::build_decoded_analysis_write(
        job,
        decoded,
        context.analysis_version,
        needs_embedding_upsert,
    )?;
    checkpoint(context)?;
    persist_decoded_analysis_write(conn, Some(job.source_root.as_path()), &write)?;
    checkpoint(context)?;
    Ok(())
}

pub(crate) fn run_analysis_jobs_with_decoded_batch(
    conn: &mut rusqlite::Connection,
    jobs: Vec<(db::ClaimedJob, wavecrate_analysis::AnalysisAudio)>,
    context: &AnalysisContext<'_>,
) -> Vec<(db::ClaimedJob, Result<(), String>)> {
    struct BatchJob {
        job: db::ClaimedJob,
        write_index: Option<usize>,
        error: Option<String>,
    }

    let job_ids: Vec<i64> = jobs.iter().map(|(job, _)| job.id).collect();
    let mut heartbeat = JobHeartbeat::new(std::time::Duration::from_secs(4));
    let mut batch_jobs = Vec::with_capacity(jobs.len());
    let mut writes = Vec::with_capacity(jobs.len());
    let _ = heartbeat.touch_jobs(conn, &job_ids);
    for (job, decoded) in jobs {
        let sample_id = job.sample_id.clone();
        let mut item = BatchJob {
            job,
            write_index: None,
            error: None,
        };
        if item.job.content_hash.as_deref().is_none() {
            item.error = Some(format!(
                "Missing content_hash for analysis job {}",
                sample_id
            ));
            batch_jobs.push(item);
            continue;
        }
        if context.use_cache {
            match load_existing_embedding(conn, &sample_id) {
                Ok(Some(_cached)) => {
                    match super::analysis_db::build_decoded_analysis_write(
                        &item.job,
                        decoded,
                        context.analysis_version,
                        false,
                    ) {
                        Ok(write) => {
                            item.write_index = Some(writes.len());
                            writes.push(write);
                        }
                        Err(err) => {
                            item.error = Some(err);
                        }
                    }
                }
                Ok(None) => {
                    match super::analysis_db::build_decoded_analysis_write(
                        &item.job,
                        decoded,
                        context.analysis_version,
                        true,
                    ) {
                        Ok(write) => {
                            item.write_index = Some(writes.len());
                            writes.push(write);
                        }
                        Err(err) => {
                            item.error = Some(err);
                        }
                    }
                }
                Err(err) => {
                    item.error = Some(err);
                }
            }
        } else {
            match super::analysis_db::build_decoded_analysis_write(
                &item.job,
                decoded,
                context.analysis_version,
                true,
            ) {
                Ok(write) => {
                    item.write_index = Some(writes.len());
                    writes.push(write);
                }
                Err(err) => {
                    item.error = Some(err);
                }
            }
        }
        batch_jobs.push(item);
    }
    let _ = heartbeat.touch_jobs(conn, &job_ids);
    let source_root = batch_jobs
        .first()
        .map(|item| item.job.source_root.as_path());
    if let Err(err) = super::analysis_db::persist_decoded_analysis_batch(conn, source_root, &writes)
    {
        return batch_jobs
            .into_iter()
            .map(|item| (item.job, Err(err.clone())))
            .collect();
    }
    let mut outcomes = Vec::with_capacity(batch_jobs.len());
    for item in batch_jobs {
        let _ = heartbeat.touch_jobs(conn, &job_ids);
        let result = item.error.map_or(Ok(()), Err);
        outcomes.push((item.job, result));
    }
    outcomes
}
