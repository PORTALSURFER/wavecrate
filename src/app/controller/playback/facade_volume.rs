use super::*;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

impl AppController {
    /// Apply the output volume multiplier immediately without persisting config.
    pub fn set_volume_live(&mut self, volume: f32) {
        transport::set_volume_live(self, volume);
    }

    /// Persist the current output volume setting if a live change is pending.
    pub fn commit_volume_setting(&mut self) {
        let started_at = Instant::now();
        let selected_source = self.selected_source_id();
        transport::commit_volume_setting(self);
        emit_action_debug_event(ActionDebugEvent {
            action: "playback.commit_volume_setting",
            pane: Some("transport"),
            source: selected_source.as_ref().map(SourceId::as_str),
            outcome: "success",
            elapsed: started_at.elapsed(),
            error: None,
        });
    }

    /// Flush a pending debounced volume-setting persistence if due.
    pub(crate) fn flush_pending_volume_setting(&mut self) {
        transport::flush_pending_volume_setting(self);
    }

    /// Return true when a deferred volume-setting persistence write is queued.
    pub(crate) fn has_pending_volume_setting_flush(&self) -> bool {
        self.runtime.config_persistence.volume_persist_dirty
    }

    /// Flush a pending deferred waveform seek commit if due.
    pub(crate) fn flush_pending_waveform_seek_commit(&mut self) {
        transport::flush_pending_waveform_seek_commit(self);
    }

    /// Return true when a deferred waveform-seek commit is queued.
    pub(crate) fn has_pending_waveform_seek_commit(&self) -> bool {
        self.runtime.waveform.pending_seek_nanos.is_some()
    }

    #[cfg(test)]
    /// Expose queued deferred waveform seek target for controller/runtime tests.
    pub(crate) fn pending_waveform_seek_nanos_for_test(&self) -> Option<u32> {
        self.runtime.waveform.pending_seek_nanos
    }
}
