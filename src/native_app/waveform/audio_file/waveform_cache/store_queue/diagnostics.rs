use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::native_app::waveform::audio_file::waveform_cache::write::StoreWriteOutcome;

pub(super) fn log_store_completion(cache_path: &Path, outcome: StoreWriteOutcome) {
    match outcome {
        StoreWriteOutcome::Completed => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_completed",
                cache_path = %cache_path.display(),
                "Completed waveform cache persistence"
            );
        }
        StoreWriteOutcome::SerializeFailed => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_serialize_failed",
                cache_path = %cache_path.display(),
                "Failed to serialize waveform cache persistence"
            );
        }
        StoreWriteOutcome::WriteFailed => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_write_failed",
                cache_path = %cache_path.display(),
                "Failed to write waveform cache persistence"
            );
        }
    }
}

pub(super) fn log_slow_cache_shutdown_flush(started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(8) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event = "browser.sample_cache.shutdown_flush",
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        "Waited for waveform cache persistence during shutdown"
    );
}
