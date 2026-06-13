use super::*;

mod focus_lockout;
mod missing_path_guards;
mod multi_selection;
mod rating_navigation;
mod rollback_recovery;
mod sidebar_assignment;

fn tag_labels(tags: Vec<crate::sample_sources::db::SourceTag>) -> Vec<String> {
    tags.into_iter().map(|tag| tag.display_label).collect()
}

/// Handles metadata queue samples.
fn metadata_queue_samples() -> Vec<crate::app::controller::batch_latency::BatchLatencySample> {
    crate::app::controller::batch_latency::snapshot()
        .into_iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect()
}
