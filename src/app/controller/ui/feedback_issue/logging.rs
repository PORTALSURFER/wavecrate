use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

pub(super) fn emit_feedback_issue_action(
    action: &'static str,
    source: Option<&str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane: Some("prompt"),
        source,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}
