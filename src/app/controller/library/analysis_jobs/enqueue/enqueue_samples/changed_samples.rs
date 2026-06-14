use super::duration_probe::update_missing_sample_durations;
use super::*;

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
    let current_version = wavecrate_analysis::analysis_version();
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
