use super::*;

fn source_id(value: &str) -> SourceId {
    SourceId::from_string(value)
}

fn progress(pending: usize, running: usize, done: usize, failed: usize) -> AnalysisProgress {
    AnalysisProgress {
        pending,
        running,
        done,
        failed,
        samples_total: pending + running + done + failed,
        samples_pending_or_running: pending + running,
    }
}

#[test]
fn global_progress_routes_to_analysis_overlay() {
    let progress = progress(2, 1, 3, 0);

    let actions = AnalysisProgressRouter::route_message(
        &AnalysisProgressRouteContext::default(),
        AnalysisJobMessage::Progress {
            source_id: None,
            progress,
        },
    );

    assert_eq!(
        actions,
        vec![AnalysisProgressRouteAction::ShowAnalysisProgress(progress)]
    );
}

#[test]
fn selected_source_progress_caches_and_updates_overlay() {
    let source_id = source_id("selected");
    let progress = progress(2, 1, 3, 0);
    let context = AnalysisProgressRouteContext {
        selected_source_id: Some(source_id.clone()),
        current_source_id: Some(source_id.clone()),
    };

    let actions = AnalysisProgressRouter::route_message(
        &context,
        AnalysisJobMessage::Progress {
            source_id: Some(source_id.clone()),
            progress,
        },
    );

    assert_eq!(
        actions,
        vec![
            AnalysisProgressRouteAction::CacheSelectedSourceProgress {
                source_id,
                progress,
            },
            AnalysisProgressRouteAction::ShowAnalysisProgress(progress),
        ]
    );
}

#[test]
fn idle_selected_progress_finalizes_selected_source() {
    let source_id = source_id("selected");
    let progress = progress(0, 0, 4, 0);
    let context = AnalysisProgressRouteContext {
        selected_source_id: Some(source_id.clone()),
        current_source_id: Some(source_id.clone()),
    };

    let actions = AnalysisProgressRouter::route_message(
        &context,
        AnalysisJobMessage::Progress {
            source_id: Some(source_id.clone()),
            progress,
        },
    );

    assert_eq!(
        actions,
        vec![
            AnalysisProgressRouteAction::CacheSelectedSourceProgress {
                source_id,
                progress,
            },
            AnalysisProgressRouteAction::QueueAnalysisFailuresRefresh,
            AnalysisProgressRouteAction::ForceSelectedFeatureCacheRefresh,
            AnalysisProgressRouteAction::ClearAnalysisProgress,
        ]
    );
}

#[test]
fn failure_messages_route_to_status_actions() {
    let actions = AnalysisProgressRouter::route_message(
        &AnalysisProgressRouteContext::default(),
        AnalysisJobMessage::EnqueueFailed("database locked".to_string()),
    );

    assert_eq!(
        actions,
        vec![AnalysisProgressRouteAction::SetStatus {
            text: "Analysis enqueue failed: database locked".to_string(),
            tone: StatusTone::Error,
        }]
    );
}
