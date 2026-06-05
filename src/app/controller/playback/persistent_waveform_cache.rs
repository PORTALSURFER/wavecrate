//! Persistent on-disk waveform decode cache reused across app restarts.
//!
//! The cache stores decoded waveform payloads and transient markers keyed by
//! source/path identity plus file metadata. This avoids repeating expensive
//! decode/transient work when the same sample is loaded again in a later app
//! session while still falling back safely when the source file changes.

use super::*;
mod entry;

use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::app_dirs;
use crate::sample_sources::SourceId;
use crate::waveform::DecodedWaveform;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use self::entry::{CACHE_VERSION, PersistentWaveformEntry};

/// Extension used for persistent waveform cache payload files.
const CACHE_FILE_EXTENSION: &str = "bin";
static PERSISTENT_WAVEFORM_CACHE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();

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
        let dir = match cache_subdir(source_id, relative_path) {
            Ok(dir) => dir,
            Err(_) => return,
        };
        if !dir.exists() {
            return;
        }
        if let Err(err) = std::fs::remove_dir_all(&dir) {
            tracing::warn!(
                "Failed to remove waveform cache directory {}: {err}",
                dir.display()
            );
        }
    }
}

/// Load one persistent waveform cache entry without requiring controller access.
pub(crate) fn load_persistent_waveform_cache_entry(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
) -> Option<PersistentWaveformHit> {
    let telemetry_enabled =
        crate::hotpath_telemetry::enabled(&PERSISTENT_WAVEFORM_CACHE_TELEMETRY_ENABLED);
    let started_at = telemetry_enabled.then(Instant::now);
    let path = cache_file_path(source_id, relative_path, metadata).ok()?;
    let read_started_at = telemetry_enabled.then(Instant::now);
    let bytes = std::fs::read(&path).ok()?;
    if let Some(read_started_at) = read_started_at {
        tracing::info!(
            target: "perf::audio_start",
            module = "persistent_waveform_cache",
            stage = "read",
            source_id = %source_id.as_str(),
            path = %relative_path.display(),
            cache_bytes = bytes.len(),
            file_size = metadata.file_size,
            elapsed_ms = read_started_at.elapsed().as_secs_f64() * 1_000.0,
            "Persistent waveform cache stage"
        );
    }
    let decode_started_at = telemetry_enabled.then(Instant::now);
    let entry: PersistentWaveformEntry = match bincode::deserialize(&bytes) {
        Ok(entry) => entry,
        Err(err) => {
            tracing::warn!(
                "Failed to decode persistent waveform cache {}: {err}",
                path.display()
            );
            let _ = std::fs::remove_file(&path);
            return None;
        }
    };
    if let Some(decode_started_at) = decode_started_at {
        tracing::info!(
            target: "perf::audio_start",
            module = "persistent_waveform_cache",
            stage = "deserialize",
            source_id = %source_id.as_str(),
            path = %relative_path.display(),
            cache_bytes = bytes.len(),
            file_size = metadata.file_size,
            elapsed_ms = decode_started_at.elapsed().as_secs_f64() * 1_000.0,
            "Persistent waveform cache stage"
        );
    }
    if entry.version() != CACHE_VERSION {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    let hit = entry.into_hit();
    if let Some(started_at) = started_at {
        tracing::info!(
            target: "perf::audio_start",
            module = "persistent_waveform_cache",
            stage = "load_hit",
            source_id = %source_id.as_str(),
            path = %relative_path.display(),
            cache_bytes = bytes.len(),
            file_size = metadata.file_size,
            decoded_samples = hit.decoded.samples.len(),
            transients = hit.transients.len(),
            elapsed_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
            "Persistent waveform cache stage"
        );
    }
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
    let path = match cache_file_path(source_id, relative_path, metadata) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "Failed to resolve waveform cache path for {}: {err}",
                relative_path.display()
            );
            return;
        }
    };
    let entry = PersistentWaveformEntry::from_runtime(decoded, transients);
    let bytes = match bincode::serialize(&entry) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(
                "Failed to encode waveform cache entry {}: {err}",
                relative_path.display()
            );
            return;
        }
    };
    if let Err(err) = write_cache_file(&path, &bytes) {
        tracing::warn!("Failed to write waveform cache {}: {err}", path.display());
        return;
    }
    if let Err(err) = cleanup_stale_cache_files(&path) {
        tracing::warn!(
            "Failed to prune stale waveform cache files in {}: {err}",
            path.display()
        );
    }
}

/// Resolve the root directory that stores all persistent waveform cache entries.
fn cache_root_dir() -> Result<PathBuf, String> {
    app_dirs::waveform_cache_dir().map_err(|err| format!("Failed to resolve waveform cache: {err}"))
}

/// Resolve the hashed directory for one source/path pair.
fn cache_subdir(source_id: &SourceId, relative_path: &Path) -> Result<PathBuf, String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source_id.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(relative_path.to_string_lossy().as_bytes());
    Ok(cache_root_dir()?.join(hasher.finalize().to_hex()))
}

/// Build the cache file path for one source/path pair and file metadata snapshot.
fn cache_file_path(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
) -> Result<PathBuf, String> {
    let file_name = format!(
        "v{CACHE_VERSION}_{}_{}.{}",
        metadata.file_size, metadata.modified_ns, CACHE_FILE_EXTENSION
    );
    Ok(cache_subdir(source_id, relative_path)?.join(file_name))
}

/// Atomically write one waveform cache payload to disk using a temporary file.
fn write_cache_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "Waveform cache path has no parent: {}",
            path.display()
        ));
    };
    std::fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;
    let tmp_path = path.with_extension(format!("{CACHE_FILE_EXTENSION}.tmp"));
    std::fs::write(&tmp_path, bytes)
        .map_err(|err| format!("Failed to write {}: {err}", tmp_path.display()))?;
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|err| format!("Failed to replace {}: {err}", path.display()))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|err| format!("Failed to rename {}: {err}", path.display()))?;
    Ok(())
}

/// Remove older cache payloads from the same hashed directory after a successful write.
fn cleanup_stale_cache_files(current_path: &Path) -> Result<(), String> {
    let Some(parent) = current_path.parent() else {
        return Ok(());
    };
    for entry in std::fs::read_dir(parent)
        .map_err(|err| format!("Failed to read {}: {err}", parent.display()))?
    {
        let entry = entry.map_err(|err| format!("Failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path == current_path {
            continue;
        }
        if path.extension() == Some(OsStr::new(CACHE_FILE_EXTENSION))
            && let Err(err) = std::fs::remove_file(&path)
        {
            return Err(format!("Failed to remove {}: {err}", path.display()));
        }
    }
    Ok(())
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
