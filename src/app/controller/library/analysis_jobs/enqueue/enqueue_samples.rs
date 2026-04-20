use super::enqueue_helpers::now_epoch_seconds;
use super::{invalidate, persist, scan};
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use crate::app::controller::library::analysis_jobs::wakeup;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use rusqlite::params;
use std::time::Instant;
use tracing::{info, warn};

struct EnqueueSamplesRequest<'a> {
    source: &'a crate::sample_sources::SampleSource,
    changed_samples: &'a [crate::sample_sources::scanner::ChangedSample],
}

pub(crate) fn enqueue_jobs_for_source(
    source: &crate::sample_sources::SampleSource,
    changed_samples: &[crate::sample_sources::scanner::ChangedSample],
) -> Result<(usize, AnalysisProgress), String> {
    let request = EnqueueSamplesRequest {
        source,
        changed_samples,
    };
    enqueue_samples(request)
}

/// Probe and store missing duration metadata for samples in a source.
/// Returns the number of samples updated.
pub(crate) fn update_missing_durations_for_source(
    source: &crate::sample_sources::SampleSource,
) -> Result<usize, String> {
    let mut conn = db::open_source_db(&source.root)?;
    let staged_samples = scan::stage_samples_for_source(source, true)?;
    update_missing_sample_durations(&mut conn, source, &staged_samples)
}

fn enqueue_samples(
    request: EnqueueSamplesRequest<'_>,
) -> Result<(usize, AnalysisProgress), String> {
    let started_at = Instant::now();
    let source_id = request.source.id.as_str();
    if request.changed_samples.is_empty() {
        let conn = db::open_source_db(&request.source.root)?;
        info!(
            "Analysis enqueue skipped: no changed samples (source_id={})",
            source_id
        );
        let progress = db::current_progress(&conn, &request.source.root)?;
        emit_action_debug_event(ActionDebugEvent {
            action: "analysis.enqueue.changed_samples",
            pane: Some("background"),
            source: Some(source_id),
            outcome: "short_circuit",
            elapsed: started_at.elapsed(),
            error: Some("no_changed_samples"),
        });
        return Ok((0, progress));
    }

    let sample_metadata =
        scan::sample_metadata_for_changed_samples(request.source, request.changed_samples);
    let mut conn = db::open_source_db(&request.source.root)?;
    let sample_ids: Vec<String> = sample_metadata
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect();
    let current_version = crate::analysis::version::analysis_version();
    let existing_states = db::sample_analysis_states(&conn, &sample_ids)?;
    let (invalidate, jobs) = invalidate::collect_changed_sample_updates(
        &sample_metadata,
        &existing_states,
        current_version,
    );

    let created_at = now_epoch_seconds();
    let (inserted, progress) = persist::write_changed_samples(
        &mut conn,
        &request.source.root,
        &sample_metadata,
        &invalidate,
        &jobs,
        request.source.id.as_str(),
        created_at,
    )?;
    if inserted > 0 {
        wakeup::notify_claim_wakeup();
    }
    if let Err(err) = update_missing_sample_durations(&mut conn, request.source, &sample_metadata) {
        warn!(
            source_id = %request.source.id,
            "Failed to update sample durations after scan: {err}"
        );
    }
    emit_action_debug_event(ActionDebugEvent {
        action: "analysis.enqueue.changed_samples",
        pane: Some("background"),
        source: Some(source_id),
        outcome: if inserted > 0 {
            "success"
        } else {
            "short_circuit"
        },
        elapsed: started_at.elapsed(),
        error: (inserted == 0).then_some("no_jobs_inserted"),
    });
    Ok((inserted, progress))
}

struct EnqueueSourceRequest<'a> {
    source: &'a crate::sample_sources::SampleSource,
}

pub(crate) fn enqueue_jobs_for_source_backfill(
    source: &crate::sample_sources::SampleSource,
) -> Result<(usize, AnalysisProgress), String> {
    let request = EnqueueSourceRequest { source };
    enqueue_source_backfill(request, false)
}

pub(crate) fn enqueue_jobs_for_source_backfill_full(
    source: &crate::sample_sources::SampleSource,
) -> Result<(usize, AnalysisProgress), String> {
    let request = EnqueueSourceRequest { source };
    enqueue_source_backfill(request, true)
}

fn enqueue_source_backfill(
    request: EnqueueSourceRequest<'_>,
    force_full: bool,
) -> Result<(usize, AnalysisProgress), String> {
    let started_at = Instant::now();
    let source_id = request.source.id.as_str();
    let mut conn = db::open_source_db(&request.source.root)?;
    let existing_jobs_total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE source_id = ?1",
            params![request.source.id.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if existing_jobs_total > 0 {
        let active_jobs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM analysis_jobs
                 WHERE source_id = ?1 AND status IN ('pending','running')",
                params![request.source.id.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if active_jobs > 0 {
            info!(
                "Analysis backfill skipped: active jobs exist (active={}, total={}, source_id={}, force_full={})",
                active_jobs, existing_jobs_total, source_id, force_full
            );
            let progress = db::current_progress(&conn, &request.source.root)?;
            emit_action_debug_event(ActionDebugEvent {
                action: "analysis.enqueue.backfill",
                pane: Some("background"),
                source: Some(source_id),
                outcome: "short_circuit",
                elapsed: started_at.elapsed(),
                error: Some("active_jobs_exist"),
            });
            return Ok((0, progress));
        }
    }
    let staged_samples = scan::stage_samples_for_source(request.source, true)?;
    if staged_samples.is_empty() {
        info!(
            "Analysis backfill skipped: no staged samples (source_id={}, force_full={})",
            source_id, force_full
        );
        let progress = db::current_progress(&conn, &request.source.root)?;
        emit_action_debug_event(ActionDebugEvent {
            action: "analysis.enqueue.backfill",
            pane: Some("background"),
            source: Some(source_id),
            outcome: "short_circuit",
            elapsed: started_at.elapsed(),
            error: Some("no_staged_samples"),
        });
        return Ok((0, progress));
    }
    let result = enqueue_from_staged_samples(
        &mut conn,
        request.source,
        staged_samples,
        db::ANALYZE_SAMPLE_JOB_TYPE,
        force_full,
        false,
        source_id,
    );
    if let Ok((inserted, _)) = &result {
        emit_action_debug_event(ActionDebugEvent {
            action: "analysis.enqueue.backfill",
            pane: Some("background"),
            source: Some(source_id),
            outcome: if *inserted > 0 {
                "success"
            } else {
                "short_circuit"
            },
            elapsed: started_at.elapsed(),
            error: (*inserted == 0).then_some("no_jobs_inserted"),
        });
    }
    result
}

struct EnqueueMissingFeaturesRequest<'a> {
    source: &'a crate::sample_sources::SampleSource,
}

pub(crate) fn enqueue_jobs_for_source_missing_features(
    source: &crate::sample_sources::SampleSource,
) -> Result<(usize, AnalysisProgress), String> {
    let request = EnqueueMissingFeaturesRequest { source };
    enqueue_missing_features(request)
}

fn enqueue_missing_features(
    request: EnqueueMissingFeaturesRequest<'_>,
) -> Result<(usize, AnalysisProgress), String> {
    let started_at = Instant::now();
    let source_id = request.source.id.as_str();
    let mut conn = db::open_source_db(&request.source.root)?;

    let staged_samples = scan::stage_samples_for_source(request.source, false)?;
    if staged_samples.is_empty() {
        let progress = db::current_progress(&conn, &request.source.root)?;
        emit_action_debug_event(ActionDebugEvent {
            action: "analysis.enqueue.missing_features",
            pane: Some("background"),
            source: Some(source_id),
            outcome: "short_circuit",
            elapsed: started_at.elapsed(),
            error: Some("no_staged_samples"),
        });
        return Ok((0, progress));
    }
    let result = enqueue_from_staged_samples(
        &mut conn,
        request.source,
        staged_samples,
        db::ANALYZE_SAMPLE_JOB_TYPE,
        false,
        true,
        source_id,
    );
    if let Ok((inserted, _)) = &result {
        emit_action_debug_event(ActionDebugEvent {
            action: "analysis.enqueue.missing_features",
            pane: Some("background"),
            source: Some(source_id),
            outcome: if *inserted > 0 {
                "success"
            } else {
                "short_circuit"
            },
            elapsed: started_at.elapsed(),
            error: (*inserted == 0).then_some("no_jobs_inserted"),
        });
    }
    result
}

fn enqueue_from_staged_samples(
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

fn update_missing_sample_durations(
    conn: &mut rusqlite::Connection,
    source: &crate::sample_sources::SampleSource,
    samples: &[db::SampleMetadata],
) -> Result<usize, String> {
    if samples.is_empty() {
        return Ok(0);
    }
    let sample_ids: Vec<String> = samples
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect();
    let missing_ids = db::sample_ids_missing_duration(conn, &sample_ids)?;
    if missing_ids.is_empty() {
        return Ok(0);
    }
    let mut updated = 0usize;
    for sample in samples {
        if !missing_ids.contains(&sample.sample_id) {
            continue;
        }
        let (_source_id, relative_path) = match db::parse_sample_id(&sample.sample_id) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Skipping duration probe for {}: {err}", sample.sample_id);
                continue;
            }
        };
        let absolute = source.root.join(&relative_path);
        let probe = match crate::analysis::audio::probe_metadata(&absolute) {
            Ok(probe) => probe,
            Err(err) => {
                warn!("Failed to probe duration for {}: {err}", absolute.display());
                continue;
            }
        };
        let Some(duration_seconds) = probe
            .duration_seconds
            .filter(|duration| duration.is_finite() && *duration > 0.0)
        else {
            continue;
        };
        let sample_rate = probe
            .sample_rate
            .unwrap_or(crate::analysis::audio::ANALYSIS_SAMPLE_RATE)
            .max(1);
        match db::update_sample_duration(conn, &sample.sample_id, duration_seconds, sample_rate) {
            Ok(true) => updated += 1,
            Ok(false) => {}
            Err(err) => {
                warn!("Failed to store duration for {}: {err}", sample.sample_id);
            }
        }
    }
    Ok(updated)
}
