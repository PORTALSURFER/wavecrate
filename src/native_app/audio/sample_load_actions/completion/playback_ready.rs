use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::{
    app::{
        NativeAppState, SampleLoadTaskCompletion, SamplePlaybackReady, emit_gui_action,
        sample_path_label,
    },
    audio::sample_load_actions::log_sample_load_timing,
    starmap_audition_telemetry::{
        self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
    },
};

impl NativeAppState {
    pub(in crate::native_app) fn finish_sample_playback_ready(
        &mut self,
        ready: SampleLoadTaskCompletion<SamplePlaybackReady>,
        context: &mut ui::UiUpdateContext<crate::native_app::app::GuiMessage>,
    ) {
        let started_at = Instant::now();
        let ticket = ready.ticket;
        let key = ready.key.clone();
        let ready = ready.output;
        let label = sample_path_label(ready.path.as_str());
        let starmap_audition_active = self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .as_deref()
                == Some(ready.path.as_str())
            || !self
                .ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .is_empty();
        let stale_gate_started_at = Instant::now();
        if !self
            .background
            .sample_load_tasks
            .is_active_key(&key, ticket)
            || self.library.folder_browser.selected_file_id() != Some(ready.path.as_str())
        {
            if starmap_audition_active {
                starmap_telemetry::record_event(
                    None,
                    "sample_load.playback_ready",
                    "stale",
                    Some(ready.path.as_str()),
                    0,
                    self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                    self.ui
                        .chrome
                        .starmap_audition_queue
                        .active_file_id
                        .is_some(),
                    None,
                );
            }
            emit_gui_action(
                "browser.sample_load.playback_ready",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        log_sample_load_timing(
            "browser.sample_load.playback_ready.stale_gate",
            &label,
            stale_gate_started_at.elapsed(),
            true,
        );
        let state_update_started_at = Instant::now();
        self.waveform
            .load
            .selection
            .playback_ready(ready.path.as_str());
        self.prepare_playback_mode_for_path(ready.path.as_str());
        log_sample_load_timing(
            "browser.sample_load.playback_ready.state_update",
            &label,
            state_update_started_at.elapsed(),
            true,
        );
        let outcome = if ready.autoplay {
            if self.start_playback_ready_instant_audition(ready.audio, context, started_at) {
                "playback_ready_playing"
            } else {
                self.ui.status.sample = format!("Preparing {label}");
                "playback_ready_waiting_for_waveform"
            }
        } else {
            "ready"
        };
        if starmap_audition_active {
            starmap_telemetry::record_event(
                Some(if outcome == "playback_ready_playing" {
                    StarmapAuditionCounter::ReadyStarted
                } else {
                    StarmapAuditionCounter::ReadyPending
                }),
                "sample_load.playback_ready",
                outcome,
                Some(ready.path.as_str()),
                0,
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                self.ui
                    .chrome
                    .starmap_audition_queue
                    .active_file_id
                    .is_some(),
                Some(started_at.elapsed()),
            );
            starmap_telemetry::record_duration(
                StarmapAuditionDuration::ReadySource,
                started_at.elapsed(),
            );
        }
        emit_gui_action(
            "browser.sample_load.playback_ready",
            Some("browser"),
            Some(&label),
            outcome,
            started_at,
            None,
        );
    }
}
