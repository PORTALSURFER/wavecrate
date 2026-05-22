use super::*;

pub(super) fn record_auto_rename_prepare_latency(
    #[cfg_attr(not(test), allow(unused_variables))] request_count: usize,
    #[cfg_attr(not(test), allow(unused_variables))] elapsed: std::time::Duration,
) {
    #[cfg(test)]
    crate::app::controller::batch_latency::record(
        crate::app::controller::batch_latency::BatchLatencySample::new(
            crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenamePrepare,
            request_count,
            elapsed,
        ),
    );
}

pub(super) fn log_prepared_auto_rename_requests(
    source: &SampleSource,
    requests: &[SampleAutoRenameRequest],
    elapsed: std::time::Duration,
    lane: &'static str,
) {
    info!(
        source_id = %source.id,
        lane,
        request_count = requests.len(),
        elapsed_ms = elapsed.as_millis() as u64,
        requests = %format_auto_rename_request_provenance(requests),
        "auto rename: request metadata provenance"
    );
}

fn format_auto_rename_request_provenance(requests: &[SampleAutoRenameRequest]) -> String {
    const MAX_ITEMS: usize = 8;
    let mut parts = requests
        .iter()
        .take(MAX_ITEMS)
        .map(|request| {
            format!(
                "{} -> {} looped={}",
                request.old_relative.display(),
                request.new_relative.display(),
                request.looped
            )
        })
        .collect::<Vec<_>>();
    if requests.len() > MAX_ITEMS {
        parts.push(format!("... +{} more", requests.len() - MAX_ITEMS));
    }
    parts.join("; ")
}
