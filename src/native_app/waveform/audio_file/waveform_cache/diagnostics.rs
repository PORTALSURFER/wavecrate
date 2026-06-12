use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn log_slow_cache_phase(event: &'static str, path: &Path, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(8) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Slow waveform cache phase"
    );
}

pub(super) fn log_stale_cache_entry(path: &Path, format_version: u32) {
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event = "browser.sample_cache.read_stale",
        source_path = %path.display(),
        cache_format_version = format_version,
        "Waveform cache entry did not match the current source identity"
    );
}
