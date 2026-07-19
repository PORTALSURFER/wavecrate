use self::router::{AnalysisRouteAction, AnalysisRouteContext};
use super::*;

mod router;

/// Apply readiness-adjacent background events to controller state.
pub(crate) fn handle_analysis_message(controller: &mut AppController, message: AnalysisJobMessage) {
    let context = AnalysisRouteContext {
        selected_source_id: controller.selection_state.ctx.selected_source.clone(),
        current_source_id: controller.current_source().map(|source| source.id.clone()),
    };
    for action in router::route_message(&context, message) {
        match action {
            AnalysisRouteAction::ForceSelectedFeatureCacheRefresh => {
                controller.force_feature_cache_refresh_for_browser();
            }
            AnalysisRouteAction::RemoveBrowserDurations(source_id) => {
                controller.ui_cache.browser.durations.remove(&source_id);
            }
            AnalysisRouteAction::RemoveFeatureCache(source_id) => {
                controller.ui_cache.browser.features.remove(&source_id);
            }
            AnalysisRouteAction::SetStatus { text, tone } => {
                controller.set_status(text, tone);
            }
        }
    }
}

#[cfg(test)]
mod tests;
