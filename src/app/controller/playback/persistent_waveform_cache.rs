//! Persistent on-disk waveform decode cache reused across app restarts.
//!
//! The cache stores decoded waveform payloads and transient markers keyed by
//! source/path identity plus file metadata. This avoids repeating expensive
//! decode/transient work when the same sample is loaded again in a later app
//! session while still falling back safely when the source file changes.

use super::*;
mod codec;
mod entry;
mod key;
mod store;
mod telemetry;

use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::sample_sources::SourceId;
use crate::waveform::DecodedWaveform;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
use self::key::cache_subdir;

/// Persistent waveform cache hit hydrated from disk and ready for controller use.
#[derive(Clone)]
pub(crate) struct PersistentWaveformHit {
    /// Decoded waveform payload restored from disk.
    pub(crate) decoded: Arc<DecodedWaveform>,
    /// Cached transient markers aligned with the decoded waveform payload.
    pub(crate) transients: Arc<[f32]>,
}

impl AppController {
    /// Load one persistent waveform cache entry when it matches the current file metadata.
    pub(crate) fn load_persistent_waveform_cache(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
        metadata: FileMetadata,
    ) -> Option<PersistentWaveformHit> {
        load_persistent_waveform_cache_entry(source_id, relative_path, metadata)
    }

    /// Persist one decoded waveform payload for future app sessions.
    pub(crate) fn persist_waveform_cache(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
        metadata: FileMetadata,
        decoded: &Arc<DecodedWaveform>,
        transients: &Arc<[f32]>,
    ) {
        persist_waveform_cache_entry(source_id, relative_path, metadata, decoded, transients)
    }

    /// Remove all persistent cache entries for one waveform path.
    pub(crate) fn invalidate_persistent_waveform_cache(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) {
        let dir = match key::cache_subdir(source_id, relative_path) {
            Ok(dir) => dir,
            Err(_) => return,
        };
        store::remove_cache_dir(&dir);
    }
}

/// Load one persistent waveform cache entry without requiring controller access.
pub(crate) fn load_persistent_waveform_cache_entry(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
) -> Option<PersistentWaveformHit> {
    let telemetry_enabled = telemetry::enabled();
    let started_at = telemetry_enabled.then(Instant::now);
    let path = key::cache_file_path(source_id, relative_path, metadata).ok()?;
    let read_started_at = telemetry_enabled.then(Instant::now);
    let bytes = store::read_cache_file(&path)?;
    telemetry::record_read_stage(
        read_started_at,
        source_id,
        relative_path,
        metadata,
        bytes.len(),
    );

    let decode_started_at = telemetry_enabled.then(Instant::now);
    let entry = codec::decode_entry(&path, &bytes)?;
    telemetry::record_deserialize_stage(
        decode_started_at,
        source_id,
        relative_path,
        metadata,
        bytes.len(),
    );

    let hit = codec::entry_into_hit_if_current(&path, entry)?;
    telemetry::record_load_hit(
        started_at,
        source_id,
        relative_path,
        metadata,
        bytes.len(),
        &hit,
    );
    Some(hit)
}

/// Persist one waveform cache entry without requiring controller access.
pub(crate) fn persist_waveform_cache_entry(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    decoded: &Arc<DecodedWaveform>,
    transients: &Arc<[f32]>,
) {
    let path = match key::cache_file_path(source_id, relative_path, metadata) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "Failed to resolve waveform cache path for {}: {err}",
                relative_path.display()
            );
            return;
        }
    };
    let bytes = match codec::encode_entry(relative_path, decoded, transients) {
        Some(bytes) => bytes,
        None => return,
    };
    if let Err(err) = store::write_cache_file(&path, &bytes) {
        tracing::warn!("Failed to write waveform cache {}: {err}", path.display());
        return;
    }
    if let Err(err) = store::cleanup_stale_cache_files(&path) {
        tracing::warn!(
            "Failed to prune stale waveform cache files in {}: {err}",
            path.display()
        );
    }
}

#[cfg(test)]
/// Regression coverage for the persistent waveform cache disk format and invalidation path.
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;
    use crate::app_dirs::ConfigBaseGuard;
    use crate::waveform::WaveformPeaks;
    use tempfile::tempdir;

    /// Build a compact decoded waveform fixture that exercises serialization fields.
    fn decoded_waveform() -> Arc<DecodedWaveform> {
        Arc::new(DecodedWaveform {
            cache_token: 7,
            samples: Arc::from(vec![0.1, -0.2, 0.3]),
            analysis_samples: Arc::from(vec![0.1, 0.3]),
            analysis_sample_rate: 22_050,
            analysis_stride: 2,
            peaks: Some(Arc::new(WaveformPeaks {
                total_frames: 3,
                channels: 1,
                bucket_size_frames: 1,
                mono: vec![(-0.2, 0.3)],
                left: None,
                right: None,
            })),
            duration_seconds: 1.5,
            sample_rate: 44_100,
            channels: 1,
        })
    }

    #[test]
    /// Persisted waveform cache entries should deserialize back into equivalent runtime payloads.
    fn persistent_waveform_cache_round_trips() {
        let root = tempdir().expect("tempdir");
        let _guard = ConfigBaseGuard::set(root.path().to_path_buf());
        let (controller, source) = dummy_controller();
        let rel = Path::new("roundtrip.wav");
        let metadata = FileMetadata {
            file_size: 12,
            modified_ns: 34,
        };
        let decoded = decoded_waveform();
        let transients: Arc<[f32]> = Arc::from(vec![0.25, 0.75]);

        controller.persist_waveform_cache(&source.id, rel, metadata, &decoded, &transients);
        let hit = controller
            .load_persistent_waveform_cache(&source.id, rel, metadata)
            .expect("persistent cache hit");

        assert_eq!(hit.transients.as_ref(), transients.as_ref());
        assert_eq!(hit.decoded.samples.as_ref(), decoded.samples.as_ref());
        assert_eq!(
            hit.decoded.analysis_samples.as_ref(),
            decoded.analysis_samples.as_ref()
        );
        assert_eq!(hit.decoded.sample_rate, decoded.sample_rate);
        assert_eq!(hit.decoded.channels, decoded.channels);
        assert_ne!(hit.decoded.cache_token, decoded.cache_token);
    }

    #[test]
    /// Invalidating one waveform path should remove its hashed cache directory from disk.
    fn invalidation_removes_persistent_waveform_cache_directory() {
        let root = tempdir().expect("tempdir");
        let _guard = ConfigBaseGuard::set(root.path().to_path_buf());
        let (controller, source) = dummy_controller();
        let rel = Path::new("stale.wav");
        let metadata = FileMetadata {
            file_size: 56,
            modified_ns: 78,
        };
        let decoded = decoded_waveform();
        let transients: Arc<[f32]> = Arc::from(vec![0.1]);

        controller.persist_waveform_cache(&source.id, rel, metadata, &decoded, &transients);
        let dir = cache_subdir(&source.id, rel).expect("cache dir");
        assert!(dir.exists());

        controller.invalidate_persistent_waveform_cache(&source.id, rel);

        assert!(!dir.exists());
    }
}
