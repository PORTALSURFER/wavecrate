use super::*;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

fn record_playback_action(
    controller: &AppController,
    action: &'static str,
    started_at: Instant,
    played: bool,
) {
    let selected_source = controller.selected_source_id();
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane: Some("transport"),
        source: selected_source.as_ref().map(SourceId::as_str),
        outcome: if played { "success" } else { "short_circuit" },
        elapsed: started_at.elapsed(),
        error: (!played).then_some("not_playable"),
    });
}

impl AppController {
    /// Start playback from the beginning of the selected sample.
    pub fn play_from_start(&mut self) -> bool {
        let started_at = Instant::now();
        let played = transport::play_from_start(self);
        record_playback_action(self, "playback.play_from_start", started_at, played);
        played
    }

    /// Start playback from the current playhead/cursor position.
    pub fn play_from_current_playhead(&mut self) -> bool {
        let started_at = Instant::now();
        let played = transport::play_from_current_playhead(self);
        record_playback_action(self, "playback.play_from_playhead", started_at, played);
        played
    }

    /// Restart playback from the last recorded start position.
    pub fn replay_from_last_start(&mut self) -> bool {
        transport::replay_from_last_start(self)
    }

    /// Start playback from the current cursor position.
    pub fn play_from_cursor(&mut self) -> bool {
        let started_at = Instant::now();
        let played = transport::play_from_cursor(self);
        record_playback_action(self, "playback.play_from_cursor", started_at, played);
        played
    }

    /// Record the most recent play start position.
    pub fn record_play_start(&mut self, position: f64) {
        transport::record_play_start(self, position);
    }

    /// Toggle between play and pause.
    pub fn toggle_play_pause(&mut self) {
        let started_at = Instant::now();
        let selected_source = self.selected_source_id();
        transport::toggle_play_pause(self);
        emit_action_debug_event(ActionDebugEvent {
            action: "playback.toggle_play_pause",
            pane: Some("transport"),
            source: selected_source.as_ref().map(SourceId::as_str),
            outcome: if self.is_playing() {
                "playing"
            } else {
                "paused"
            },
            elapsed: started_at.elapsed(),
            error: None,
        });
    }

    /// Stop playback if it is currently active.
    pub fn stop_playback_if_active(&mut self) -> bool {
        transport::stop_playback_if_active(self)
    }

    /// Handle escape key behavior for playback and selection.
    pub fn handle_escape(&mut self) {
        transport::handle_escape(self);
    }
}
