use super::duration_probe::update_missing_sample_durations;
use super::*;

pub(super) fn enqueue_from_staged_samples(
    conn: &mut rusqlite::Connection,
    source: &crate::sample_sources::SampleSource,
    staged_samples: Vec<db::SampleMetadata>,
    job_type: &str,
    force_full: bool,
    skip_when_no_jobs: bool,
    source_id: &str,
) -> Result<(usize, AnalysisProgress), String> {
    if staged_samples.is_empty() {
        return Ok((0, db::current_progress(conn, &source.root)?));
    }
    persist::stage_backfill_samples(conn, &staged_samples)?;
    let plan =
        invalidate::build_backfill_plan(conn, &staged_samples, job_type, force_full, source_id)?;

    if skip_when_no_jobs && plan.jobs.is_empty() {
        info!(
            "Analysis backfill: no jobs to enqueue (staged={}, failed_requeued={}, source_id={}, job_type={}, force_full={})",
            staged_samples.len(),
            plan.failed_requeued,
            source_id,
            job_type,
            force_full
        );
        return Ok((0, db::current_progress(conn, &source.root)?));
    }
    info!(
        "Analysis backfill prepared (staged={}, jobs={}, failed_requeued={}, invalidate={}, source_id={}, job_type={}, force_full={})",
        staged_samples.len(),
        plan.jobs.len(),
        plan.failed_requeued,
        plan.invalidate.len(),
        source_id,
        job_type,
        force_full
    );
    let created_at = now_epoch_seconds();
    let (inserted, progress) = persist::write_backfill_samples(
        conn,
        &source.root,
        &plan.sample_metadata,
        &plan.invalidate,
        &plan.jobs,
        job_type,
        source_id,
        created_at,
    )?;
    if inserted > 0 {
        wakeup::notify_claim_wakeup();
    }
    if let Err(err) = update_missing_sample_durations(conn, source, &plan.sample_metadata) {
        warn!(
            source_id = %source.id,
            "Failed to update sample durations during backfill: {err}"
        );
    }
    info!(
        "Analysis backfill enqueued (inserted={}, staged={}, jobs={}, failed_requeued={}, source_id={}, job_type={})",
        inserted,
        staged_samples.len(),
        plan.jobs.len(),
        plan.failed_requeued,
        source_id,
        job_type
    );
    Ok((inserted, progress))
}
