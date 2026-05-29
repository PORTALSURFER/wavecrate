use super::{random_nav, *};
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::path::{Path, PathBuf};
use std::time::Instant;

impl AppController {
    /// Jump to a random visible sample in the browser and start playback.
    pub fn play_random_visible_sample(&mut self) {
        let started_at = Instant::now();
        let selected_source = self.selected_source_id();
        random_nav::play_random_visible_sample(self);
        emit_random_navigation_action(
            "playback.play_random_visible_sample",
            selected_source.as_ref(),
            started_at,
        );
    }

    #[cfg(test)]
    pub(crate) fn play_random_visible_sample_with_seed(&mut self, seed: u64) {
        random_nav::play_random_visible_sample_with_seed(self, seed);
    }

    /// Focus a random visible sample without starting playback.
    pub fn focus_random_visible_sample(&mut self) {
        random_nav::focus_random_visible_sample(self);
    }

    /// Resolve the next random visible sample path without changing browser focus.
    pub(crate) fn next_random_visible_sample_path(&mut self) -> Option<PathBuf> {
        random_nav::next_random_visible_sample_path(self)
    }

    /// Record one chosen random-navigation destination in the visit/history state.
    pub(crate) fn record_random_navigation_target_for_source(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
    ) {
        random_nav::record_random_navigation_target_for_source(self, source_id, relative_path);
    }

    /// Play the previous entry from the random history stack.
    pub fn play_previous_random_sample(&mut self) {
        let started_at = Instant::now();
        let selected_source = self.selected_source_id();
        random_nav::play_previous_random_sample(self);
        emit_random_navigation_action(
            "playback.play_previous_random_sample",
            selected_source.as_ref(),
            started_at,
        );
    }

    /// Toggle sticky random navigation for Up/Down in the browser.
    pub fn toggle_random_navigation_mode(&mut self) {
        random_nav::toggle_random_navigation_mode(self);
    }

    /// Return whether sticky random navigation mode is enabled.
    pub fn random_navigation_mode_enabled(&self) -> bool {
        random_nav::random_navigation_mode_enabled(self)
    }
}

fn emit_random_navigation_action(
    action: &'static str,
    selected_source: Option<&SourceId>,
    started_at: Instant,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane: Some("browser"),
        source: selected_source.map(SourceId::as_str),
        outcome: "success",
        elapsed: started_at.elapsed(),
        error: None,
    });
}
