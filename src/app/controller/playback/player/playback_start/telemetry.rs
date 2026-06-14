use super::*;
use crate::app::controller::playback::telemetry::log_audio_start_stage;

pub(super) type StageStart = Option<std::time::Instant>;

pub(super) fn stage_timer() -> StageStart {
    crate::app::controller::playback::telemetry::stage_timer()
}

pub(super) struct LoadedAudioTelemetry {
    source_id: Option<SourceId>,
    relative_path: Option<PathBuf>,
    source_kind: Option<&'static str>,
    byte_len: Option<usize>,
}

impl LoadedAudioTelemetry {
    pub(super) fn from_controller(controller: &AppController) -> Self {
        let loaded = controller.sample_view.wav.loaded_audio.as_ref();
        Self {
            source_id: loaded.map(|audio| audio.source_id.clone()),
            relative_path: loaded.map(|audio| audio.relative_path.clone()),
            source_kind: loaded.map(|audio| {
                if audio.bytes.is_empty() {
                    "file"
                } else {
                    "bytes"
                }
            }),
            byte_len: loaded.map(|audio| audio.bytes.len()),
        }
    }
}

pub(super) fn log_pending_load_stage(
    action: &'static str,
    started_at: StageStart,
    source_id: Option<&SourceId>,
    relative_path: Option<&PathBuf>,
) {
    log_audio_start_stage(
        action,
        source_id,
        relative_path.map(PathBuf::as_path),
        started_at,
        None,
        Some("pending_load"),
        None,
        None,
    );
}

pub(super) fn log_playback_stage(
    action: &'static str,
    telemetry: &LoadedAudioTelemetry,
    started_at: StageStart,
    outcome: Option<&'static str>,
) {
    log_audio_start_stage(
        action,
        telemetry.source_id.as_ref(),
        telemetry.relative_path.as_deref(),
        started_at,
        telemetry.source_kind,
        outcome,
        telemetry.byte_len,
        None,
    );
}
