use super::super::*;
use crate::app::controller::playback::telemetry::log_audio_start_stage;

mod cache;
mod error;
mod handoff;
mod transients;
mod visual;

impl AppController {
    pub(crate) fn handle_audio_loaded(&mut self, pending: PendingAudio, outcome: AudioLoadOutcome) {
        log_audio_start_stage(
            "handle_audio_loaded",
            Some(&pending.source_id),
            Some(&pending.relative_path),
            None,
            Some(if outcome.bytes.is_empty() {
                "file"
            } else {
                "bytes"
            }),
            None,
            Some(outcome.bytes.len()),
            Some(outcome.decoded.samples.len()),
        );
        let duration_seconds = outcome.decoded.duration_seconds;
        let sample_rate = outcome.decoded.sample_rate;
        self.runtime
            .jobs
            .set_staged_audio_handoff(Some(StagedAudioHandoff {
                request_id: pending.request_id,
                source_id: pending.source_id.clone(),
                root: pending.root,
                relative_path: pending.relative_path.clone(),
                intent: pending.intent,
                decoded: outcome.decoded,
                bytes: outcome.bytes,
                audio_path: outcome.audio_path,
            }));
        self.note_browser_selection_staged(&pending.source_id, &pending.relative_path);
        let message =
            Self::loaded_status_text(&pending.relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
    }
}
