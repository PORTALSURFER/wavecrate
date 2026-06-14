use super::PersistentWaveformHit;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::sample_sources::SourceId;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

static PERSISTENT_WAVEFORM_CACHE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();

pub(super) fn enabled() -> bool {
    crate::hotpath_telemetry::enabled(&PERSISTENT_WAVEFORM_CACHE_TELEMETRY_ENABLED)
}

pub(super) fn record_read_stage(
    started_at: Option<Instant>,
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    cache_bytes: usize,
) {
    record_cache_stage(
        started_at,
        "read",
        source_id,
        relative_path,
        metadata,
        cache_bytes,
    );
}

pub(super) fn record_deserialize_stage(
    started_at: Option<Instant>,
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    cache_bytes: usize,
) {
    record_cache_stage(
        started_at,
        "deserialize",
        source_id,
        relative_path,
        metadata,
        cache_bytes,
    );
}

pub(super) fn record_load_hit(
    started_at: Option<Instant>,
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    cache_bytes: usize,
    hit: &PersistentWaveformHit,
) {
    let Some(started_at) = started_at else {
        return;
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "persistent_waveform_cache",
        stage = "load_hit",
        source_id = %source_id.as_str(),
        path = %relative_path.display(),
        cache_bytes,
        file_size = metadata.file_size,
        decoded_samples = hit.decoded.samples.len(),
        transients = hit.transients.len(),
        elapsed_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
        "Persistent waveform cache stage"
    );
}

fn record_cache_stage(
    started_at: Option<Instant>,
    stage: &'static str,
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    cache_bytes: usize,
) {
    let Some(started_at) = started_at else {
        return;
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "persistent_waveform_cache",
        stage,
        source_id = %source_id.as_str(),
        path = %relative_path.display(),
        cache_bytes,
        file_size = metadata.file_size,
        elapsed_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
        "Persistent waveform cache stage"
    );
}
