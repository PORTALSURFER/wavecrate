use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::native_app::waveform::audio_file::waveform_cache::write::StoreWriteOutcome;

pub(super) fn log_store_completion(cache_path: &Path, outcome: StoreWriteOutcome) {
    if matches!(outcome, StoreWriteOutcome::Completed(_)) && !outcome.report().has_failures() {
        tracing::debug!(
            target: "wavecrate::debug::sample_cache",
            event = "browser.sample_cache.store_completed",
            cache_path = %cache_path.display(),
            outcome = outcome.kind(),
            report = ?outcome.report(),
            "Completed waveform cache persistence"
        );
        return;
    }
    match outcome {
        StoreWriteOutcome::Completed(_) => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_completed_with_diagnostics",
                cache_path = %cache_path.display(),
                outcome = outcome.kind(),
                report = ?outcome.report(),
                "Completed waveform cache persistence with write-side diagnostics"
            );
        }
        StoreWriteOutcome::StaleInput(_) => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_stale_input",
                cache_path = %cache_path.display(),
                outcome = outcome.kind(),
                report = ?outcome.report(),
                "Skipped stale waveform cache persistence"
            );
        }
        StoreWriteOutcome::SerializeFailed(_)
        | StoreWriteOutcome::TempWriteFailed(_)
        | StoreWriteOutcome::RenameFailed(_) => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_failed",
                cache_path = %cache_path.display(),
                outcome = outcome.kind(),
                report = ?outcome.report(),
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
