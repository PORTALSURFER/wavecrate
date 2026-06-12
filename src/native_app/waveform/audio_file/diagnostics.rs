use std::{path::Path, time::Duration};

pub(super) fn log_audio_load_timing(event: &'static str, path: &Path, elapsed: Duration) {
    tracing::info!(
        target: "wavecrate::debug::sample_load",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Audio file load timing"
    );
}
