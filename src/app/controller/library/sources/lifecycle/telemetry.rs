use super::*;

pub(super) fn record_source_lifecycle_event(
    action: &'static str,
    source: Option<&str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane: Some("sources"),
        source,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}
