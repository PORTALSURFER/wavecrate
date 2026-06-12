use std::time::{Duration, Instant};

pub(super) fn log_slow_playback_phase(
    event: &'static str,
    file_name: &str,
    source_kind: &'static str,
    started_at: Instant,
) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(4) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::playback",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        file_name,
        source_kind,
        "Slow playback UI phase"
    );
}
