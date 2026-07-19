use crate::app::controller::library::analysis_jobs::AnalysisJobMessage;
use crate::app_core::state::StatusTone;
use crate::sample_sources::SourceId;

#[derive(Clone, Debug, Default)]
pub(super) struct AnalysisRouteContext {
    pub(super) selected_source_id: Option<SourceId>,
    pub(super) current_source_id: Option<SourceId>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum AnalysisRouteAction {
    ForceSelectedFeatureCacheRefresh,
    RemoveBrowserDurations(SourceId),
    RemoveFeatureCache(SourceId),
    SetStatus { text: String, tone: StatusTone },
}

pub(super) fn route_message(
    context: &AnalysisRouteContext,
    message: AnalysisJobMessage,
) -> Vec<AnalysisRouteAction> {
    match message {
        AnalysisJobMessage::ReadinessReconciliationFinished {
            source_id,
            changed,
            announce,
        } => route_reconciliation_finished(context, source_id, changed, announce),
        AnalysisJobMessage::ReadinessReconciliationFailed(err) => {
            vec![AnalysisRouteAction::SetStatus {
                text: format!("Readiness reconciliation failed: {err}"),
                tone: StatusTone::Error,
            }]
        }
        AnalysisJobMessage::DurationsUpdated { source_id, updated } => {
            route_durations_updated(context, source_id, updated)
        }
    }
}

fn route_reconciliation_finished(
    context: &AnalysisRouteContext,
    source_id: SourceId,
    changed: usize,
    announce: bool,
) -> Vec<AnalysisRouteAction> {
    let mut actions = Vec::new();
    if changed > 0 && announce {
        actions.push(AnalysisRouteAction::SetStatus {
            text: format!("Queued readiness reconciliation for {changed} samples"),
            tone: StatusTone::Info,
        });
    }
    if context.selected_source_id.as_ref() == Some(&source_id) {
        if context.current_source_id.as_ref() == Some(&source_id) {
            actions.push(AnalysisRouteAction::ForceSelectedFeatureCacheRefresh);
        } else {
            actions.push(AnalysisRouteAction::RemoveFeatureCache(source_id));
        }
    }
    actions
}

fn route_durations_updated(
    context: &AnalysisRouteContext,
    source_id: SourceId,
    updated: usize,
) -> Vec<AnalysisRouteAction> {
    if updated == 0 {
        return Vec::new();
    }
    let mut actions = if context.selected_source_id.as_ref() == Some(&source_id) {
        vec![AnalysisRouteAction::ForceSelectedFeatureCacheRefresh]
    } else {
        vec![AnalysisRouteAction::RemoveFeatureCache(source_id.clone())]
    };
    actions.push(AnalysisRouteAction::RemoveBrowserDurations(source_id));
    actions
}

#[cfg(test)]
mod tests;
