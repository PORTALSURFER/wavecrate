use super::staged_samples::enqueue_from_staged_samples;
use super::*;

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
