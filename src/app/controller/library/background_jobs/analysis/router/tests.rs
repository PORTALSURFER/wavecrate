use super::*;

fn source_id(value: &str) -> SourceId {
    SourceId::from_string(value)
}

#[test]
fn reconciliation_refreshes_the_selected_source() {
    let selected = source_id("selected");
    let context = AnalysisRouteContext {
        selected_source_id: Some(selected.clone()),
        current_source_id: Some(selected.clone()),
    };

    let actions = route_message(
        &context,
        AnalysisJobMessage::ReadinessReconciliationFinished {
            source_id: selected,
            changed: 2,
            announce: true,
        },
    );

    assert_eq!(
        actions,
        vec![
            AnalysisRouteAction::SetStatus {
                text: "Queued readiness reconciliation for 2 samples".to_string(),
                tone: StatusTone::Info,
            },
            AnalysisRouteAction::ForceSelectedFeatureCacheRefresh,
        ]
    );
}

#[test]
fn reconciliation_failure_reports_readiness_context() {
    let actions = route_message(
        &AnalysisRouteContext::default(),
        AnalysisJobMessage::ReadinessReconciliationFailed("database locked".to_string()),
    );

    assert_eq!(
        actions,
        vec![AnalysisRouteAction::SetStatus {
            text: "Readiness reconciliation failed: database locked".to_string(),
            tone: StatusTone::Error,
        }]
    );
}

#[test]
fn duration_updates_invalidate_the_source_cache() {
    let source = source_id("other");
    let actions = route_message(
        &AnalysisRouteContext::default(),
        AnalysisJobMessage::DurationsUpdated {
            source_id: source.clone(),
            updated: 1,
        },
    );

    assert_eq!(
        actions,
        vec![
            AnalysisRouteAction::RemoveFeatureCache(source.clone()),
            AnalysisRouteAction::RemoveBrowserDurations(source),
        ]
    );
}
