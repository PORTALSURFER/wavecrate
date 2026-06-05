//! Opt-in audio-start diagnostics for controller playback workflows.

use crate::hotpath_telemetry;
use crate::sample_sources::SourceId;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static AUDIO_START_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();

pub(crate) fn audio_start_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&AUDIO_START_TELEMETRY_ENABLED)
}

pub(crate) fn stage_timer() -> Option<Instant> {
    audio_start_telemetry_enabled().then(Instant::now)
}

pub(crate) fn log_audio_start_stage(
    stage: &'static str,
    source_id: Option<&SourceId>,
    relative_path: Option<&Path>,
    started_at: Option<Instant>,
    source_kind: Option<&'static str>,
    cache_state: Option<&'static str>,
    byte_len: Option<usize>,
    sample_len: Option<usize>,
) {
    if !audio_start_telemetry_enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "wavecrate_controller",
        stage,
        source_id = source_id.map(SourceId::as_str).unwrap_or(""),
        path = %relative_path.map(|path| path.display().to_string()).unwrap_or_default(),
        source_kind = source_kind.unwrap_or(""),
        cache_state = cache_state.unwrap_or(""),
        byte_len = byte_len.unwrap_or(0),
        sample_len = sample_len.unwrap_or(0),
        elapsed_ms = started_at.map(elapsed_ms).unwrap_or(0.0),
        "Controller audio-start stage"
    );
}

fn elapsed_ms(started_at: Instant) -> f64 {
    duration_ms(started_at.elapsed())
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
