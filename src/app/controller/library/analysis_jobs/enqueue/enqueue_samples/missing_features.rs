use super::staged_samples::enqueue_from_staged_samples;
use super::*;

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
