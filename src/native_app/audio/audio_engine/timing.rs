use std::time::Duration;

pub(super) fn log_audio_open_timing(event: &'static str, elapsed: Duration, always: bool) {
    if !always && elapsed < Duration::from_millis(4) {
        return;
    }
    if always {
        tracing::info!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            "Audio output timing"
        );
    } else {
        tracing::warn!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            "Slow audio output UI phase"
        );
    }
}
