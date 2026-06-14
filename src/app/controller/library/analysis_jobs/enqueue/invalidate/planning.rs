use super::model::{BackfillPlan, BackfillUpdates, QueuedBackfillJobs};
use super::queries::{
    fetch_backfill_invalidations, fetch_backfill_jobs, fetch_failed_backfill_jobs,
    fetch_force_backfill_jobs,
};
use crate::app::controller::library::analysis_jobs::db;
use std::collections::{HashMap, HashSet};

pub(crate) fn collect_changed_sample_updates(
    sample_metadata: &[db::SampleMetadata],
    existing_states: &HashMap<String, db::SampleAnalysisState>,
    current_version: &str,
) -> (Vec<String>, Vec<(String, String)>) {
    let mut invalidate = Vec::new();
    let mut jobs = Vec::new();
    for sample in sample_metadata {
        let state = existing_states.get(&sample.sample_id);
        let hash_changed = state
            .map(|state| state.content_hash != sample.content_hash)
            .unwrap_or(true);
        let analysis_stale = state
            .and_then(|state| state.analysis_version.as_deref())
            .map(|version| version != current_version)
            .unwrap_or(true);
        if hash_changed || analysis_stale {
            invalidate.push(sample.sample_id.clone());
            jobs.push((sample.sample_id.clone(), sample.content_hash.clone()));
        }
    }
    (invalidate, jobs)
}

pub(crate) fn collect_backfill_updates(
    conn: &mut rusqlite::Connection,
    job_type: &str,
    force_full: bool,
) -> Result<BackfillUpdates, String> {
    if force_full {
        let QueuedBackfillJobs {
            sample_metadata,
            jobs,
        } = fetch_force_backfill_jobs(conn, job_type)?;
        return Ok(BackfillUpdates {
            sample_metadata,
            jobs,
            invalidate: Vec::new(),
        });
    }
    let current_version = wavecrate_analysis::analysis_version();
    let invalidate = fetch_backfill_invalidations(conn, current_version)?;
    let QueuedBackfillJobs {
        sample_metadata,
        jobs,
    } = fetch_backfill_jobs(
        conn,
        current_version,
        job_type,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
    )?;
    Ok(BackfillUpdates {
        sample_metadata,
        jobs,
        invalidate,
    })
}

pub(crate) fn build_backfill_plan(
    conn: &mut rusqlite::Connection,
    staged_samples: &[db::SampleMetadata],
    job_type: &str,
    force_full: bool,
    source_id: &str,
) -> Result<BackfillPlan, String> {
    let staged_index: HashMap<String, db::SampleMetadata> = staged_samples
        .iter()
        .map(|sample| (sample.sample_id.clone(), sample.clone()))
        .collect();
    let BackfillUpdates {
        mut sample_metadata,
        mut jobs,
        mut invalidate,
    } = collect_backfill_updates(conn, job_type, force_full)?;
    let failed_jobs = if force_full {
        fetch_failed_backfill_jobs(conn, job_type, source_id)?
    } else {
        Vec::new()
    };
    let failed_requeued = merge_failed_backfill_jobs(
        &staged_index,
        &mut sample_metadata,
        &mut jobs,
        &mut invalidate,
        &failed_jobs,
    );
    if !invalidate.is_empty() {
        invalidate.sort();
        invalidate.dedup();
    }
    Ok(BackfillPlan {
        sample_metadata,
        jobs,
        invalidate,
        failed_requeued,
    })
}

pub(crate) fn merge_failed_backfill_jobs(
    staged_index: &HashMap<String, db::SampleMetadata>,
    sample_metadata: &mut Vec<db::SampleMetadata>,
    jobs: &mut Vec<(String, String)>,
    invalidate: &mut Vec<String>,
    failed_jobs: &[String],
) -> usize {
    if failed_jobs.is_empty() {
        return 0;
    }
    let mut job_ids: HashSet<String> = jobs.iter().map(|(id, _)| id.clone()).collect();
    let mut sample_ids: HashSet<String> = sample_metadata
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect();
    let mut failed_count = 0;
    for sample_id in failed_jobs {
        let Some(sample) = staged_index.get(sample_id) else {
            continue;
        };
        if job_ids.insert(sample_id.clone()) {
            jobs.push((sample_id.clone(), sample.content_hash.clone()));
            failed_count += 1;
        }
        if sample_ids.insert(sample_id.clone()) {
            sample_metadata.push(sample.clone());
        }
        invalidate.push(sample_id.clone());
    }
    failed_count
}
