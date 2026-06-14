use super::super::super::*;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

impl AppController {
    /// Refresh the wav list for the selected source (delegates to background load).
    pub fn refresh_wavs(&mut self) -> Result<(), SourceDbError> {
        let started_at = Instant::now();
        let selected_source = self.selected_source_id();
        self.queue_wav_load();
        emit_action_debug_event(ActionDebugEvent {
            action: "sources.refresh_wavs",
            pane: Some("browser"),
            source: selected_source.as_ref().map(SourceId::as_str),
            outcome: "queued",
            elapsed: started_at.elapsed(),
            error: None,
        });
        Ok(())
    }
}
