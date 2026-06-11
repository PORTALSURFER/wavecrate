use crate::app::controller::library::analysis_jobs::{AnalysisJobMessage, AnalysisProgress};
use crate::app_core::state::StatusTone;
use crate::sample_sources::SourceId;

#[derive(Clone, Debug, Default)]
pub(super) struct AnalysisProgressRouteContext {
    pub(super) selected_source_id: Option<SourceId>,
    pub(super) current_source_id: Option<SourceId>,
    pub(super) similarity_prep_source_id: Option<SourceId>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum AnalysisProgressRouteAction {
    CacheSelectedSourceProgress {
        source_id: SourceId,
        progress: AnalysisProgress,
    },
    ClearAnalysisProgress,
    ForwardSimilarityPrepProgress(AnalysisProgress),
    ForceSelectedFeatureCacheRefresh,
    QueueAnalysisFailuresRefresh,
    QueueSelectedSourceProgress(AnalysisProgress),
    RemoveBrowserDurations(SourceId),
    RemoveFeatureCache(SourceId),
    ResumeAnalysis,
    SetStatus {
        text: String,
        tone: StatusTone,
    },
    ShowAnalysisProgress(AnalysisProgress),
}

pub(super) struct AnalysisProgressRouter;

impl AnalysisProgressRouter {
    pub(super) fn route_message(
        context: &AnalysisProgressRouteContext,
        message: AnalysisJobMessage,
    ) -> Vec<AnalysisProgressRouteAction> {
        match message {
            AnalysisJobMessage::Progress {
                source_id,
                progress,
            } => route_progress(context, source_id, progress),
            AnalysisJobMessage::EnqueueFinished {
                inserted,
                progress,
                announce,
            } => route_enqueue_finished(context, inserted, progress, false, announce),
            AnalysisJobMessage::EnqueueFailed(err) => {
                vec![AnalysisProgressRouteAction::SetStatus {
                    text: format!("Analysis enqueue failed: {err}"),
                    tone: StatusTone::Error,
                }]
            }
            AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
                inserted,
                progress,
                announce,
            } => route_enqueue_finished(context, inserted, progress, true, announce),
            AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err) => {
                vec![AnalysisProgressRouteAction::SetStatus {
                    text: format!("Embedding backfill enqueue failed: {err}"),
                    tone: StatusTone::Error,
                }]
            }
            AnalysisJobMessage::DurationsUpdated { source_id, updated } => {
                route_durations_updated(context, source_id, updated)
            }
        }
    }
}

fn route_progress(
    context: &AnalysisProgressRouteContext,
    source_id: Option<SourceId>,
    progress: AnalysisProgress,
) -> Vec<AnalysisProgressRouteAction> {
    if context.similarity_prep_source_id.is_some()
        && source_id.as_ref() != context.similarity_prep_source_id.as_ref()
    {
        return Vec::new();
    }

    let mut actions = Vec::new();
    if source_id.as_ref() == context.selected_source_id.as_ref()
        && let Some(source_id) = source_id.as_ref()
    {
        actions.push(AnalysisProgressRouteAction::CacheSelectedSourceProgress {
            source_id: source_id.clone(),
            progress,
        });
    }
    if source_id.as_ref() == context.similarity_prep_source_id.as_ref() && source_id.is_some() {
        actions.push(AnalysisProgressRouteAction::ForwardSimilarityPrepProgress(
            progress,
        ));
    }
    if !progress_matches_selected_source(context.selected_source_id.as_ref(), source_id.as_ref()) {
        return actions;
    }
    if progress.total() == 0 {
        actions.push(AnalysisProgressRouteAction::ClearAnalysisProgress);
        return actions;
    }
    if progress_is_idle(&progress) {
        actions.push(AnalysisProgressRouteAction::QueueAnalysisFailuresRefresh);
        actions.push(AnalysisProgressRouteAction::ForceSelectedFeatureCacheRefresh);
        actions.push(AnalysisProgressRouteAction::ClearAnalysisProgress);
        return actions;
    }
    actions.push(AnalysisProgressRouteAction::ShowAnalysisProgress(progress));
    actions
}

fn route_enqueue_finished(
    context: &AnalysisProgressRouteContext,
    inserted: usize,
    progress: AnalysisProgress,
    embedding_backfill: bool,
    announce: bool,
) -> Vec<AnalysisProgressRouteAction> {
    let mut actions = vec![AnalysisProgressRouteAction::ResumeAnalysis];
    if inserted > 0 && announce {
        let label = if embedding_backfill {
            "embedding backfill jobs"
        } else {
            "analysis jobs"
        };
        actions.push(AnalysisProgressRouteAction::SetStatus {
            text: format!("Queued {inserted} {label}"),
            tone: StatusTone::Info,
        });
    }
    if !embedding_backfill && let Some(source_id) = context.selected_source_id.as_ref() {
        if context.current_source_id.as_ref() == Some(source_id) {
            actions.push(AnalysisProgressRouteAction::ForceSelectedFeatureCacheRefresh);
        } else {
            actions.push(AnalysisProgressRouteAction::RemoveFeatureCache(
                source_id.clone(),
            ));
        }
    }
    actions.push(AnalysisProgressRouteAction::QueueSelectedSourceProgress(
        progress,
    ));
    actions
}

fn route_durations_updated(
    context: &AnalysisProgressRouteContext,
    source_id: SourceId,
    updated: usize,
) -> Vec<AnalysisProgressRouteAction> {
    if updated == 0 {
        return Vec::new();
    }
    let mut actions = if context.selected_source_id.as_ref() == Some(&source_id) {
        vec![AnalysisProgressRouteAction::ForceSelectedFeatureCacheRefresh]
    } else {
        vec![AnalysisProgressRouteAction::RemoveFeatureCache(
            source_id.clone(),
        )]
    };
    actions.push(AnalysisProgressRouteAction::RemoveBrowserDurations(
        source_id,
    ));
    actions
}

fn progress_matches_selected_source(
    selected_source: Option<&SourceId>,
    source_id: Option<&SourceId>,
) -> bool {
    match source_id {
        None => true,
        Some(id) => selected_source.is_some_and(|selected| selected == id),
    }
}

fn progress_is_idle(progress: &AnalysisProgress) -> bool {
    progress.pending == 0 && progress.running == 0
}

#[cfg(test)]
mod tests;
