use std::time::{Duration, Instant};

use crate::native_app::app::WaveformState;

pub(crate) fn log_slow_sample_load_phase(event: &'static str, source: &str, started_at: Instant) {
    let elapsed = started_at.elapsed();
    log_sample_load_timing(event, source, elapsed, false);
}

pub(crate) fn log_sample_load_timing(
    event: &'static str,
    source: &str,
    elapsed: Duration,
    always: bool,
) {
    if !always && elapsed < Duration::from_millis(4) {
        return;
    }
    if always {
        tracing::info!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            source,
            "Sample load timing"
        );
    } else {
        tracing::warn!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            source,
            "Slow sample load UI phase"
        );
    }
}

pub(crate) fn log_loaded_sample_metadata(
    source: &str,
    result: &Result<WaveformState, String>,
    cache_state: &'static str,
) {
    let Ok(waveform) = result else {
        return;
    };
    tracing::info!(
        target: "wavecrate::debug::sample_load",
        event = "browser.sample_load.worker.loaded_metadata",
        source,
        cache_state,
        sample_rate = waveform.sample_rate(),
        channels = waveform.channels(),
        frames = waveform.frames(),
        file_size_bytes = waveform.audio_bytes().len(),
        file_backed_playback = waveform.playback_source_file().is_some(),
        playback_ready = waveform.playback_samples().is_some()
            || waveform.playback_cache_file().is_some()
            || waveform.playback_source_file().is_some(),
        "Loaded sample metadata"
    );
}
