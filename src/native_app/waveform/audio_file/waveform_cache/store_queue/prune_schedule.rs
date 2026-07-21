use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::diagnostics::log_prune_completion;
use crate::native_app::waveform::audio_file::waveform_cache::{
    MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
    identity::{playback_descriptor_path, playback_sidecar_path},
    prune::prune_waveform_cache_dir,
};

use super::CachedWaveformStoreJob;

pub(super) const MAX_WRITES_BETWEEN_PRUNES: usize = 64;
pub(super) const MAX_BYTES_BETWEEN_PRUNES: u64 = 16 * 1024 * 1024;

pub(super) enum StoreWorkerAction {
    Write(CachedWaveformStoreJob),
    Prune,
}

#[derive(Debug, Default)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) struct CachePruneSchedule {
    successful_writes: usize,
    bytes_written: u64,
    pinned_path: Option<PathBuf>,
}

impl CachePruneSchedule {
    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn record_success(
        &mut self,
        cache_path: &Path,
        written_bytes: Option<u64>,
    ) {
        self.successful_writes = self.successful_writes.saturating_add(1);
        self.bytes_written = self
            .bytes_written
            .saturating_add(written_bytes.unwrap_or(MAX_BYTES_BETWEEN_PRUNES));
        self.pinned_path = Some(cache_path.to_path_buf());
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn immediate_prune_due(
        &self,
    ) -> bool {
        self.successful_writes >= MAX_WRITES_BETWEEN_PRUNES
            || self.bytes_written >= MAX_BYTES_BETWEEN_PRUNES
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn pinned_path(
        &self,
    ) -> Option<&Path> {
        self.pinned_path.as_deref()
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn successful_writes(
        &self,
    ) -> usize {
        self.successful_writes
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn reset(&mut self) {
        *self = Self::default();
    }
}

pub(super) fn reconcile_cache(
    pinned_path: Option<&Path>,
    successful_writes: usize,
    bytes_written: u64,
    reason: &'static str,
) {
    let cache_dir = pinned_path
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| wavecrate::app_dirs::waveform_cache_dir().ok());
    let Some(cache_dir) = cache_dir else {
        tracing::warn!(
            target: "wavecrate::debug::sample_cache",
            event = "browser.sample_cache.prune_dir_unavailable",
            reason,
            "Failed to resolve the waveform cache directory for pruning"
        );
        return;
    };
    let outcome =
        prune_waveform_cache_dir(&cache_dir, pinned_path, MAX_PERSISTED_WAVEFORM_CACHE_BYTES);
    log_prune_completion(
        &cache_dir,
        reason,
        successful_writes,
        bytes_written,
        outcome,
    );
}

pub(super) fn published_cache_bytes(cache_path: &Path) -> Option<u64> {
    let mut bytes = required_file_bytes(cache_path)?;
    for companion in [
        playback_sidecar_path(cache_path),
        playback_descriptor_path(cache_path),
    ] {
        bytes = bytes.saturating_add(optional_file_bytes(&companion)?);
    }
    Some(bytes)
}

fn required_file_bytes(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
}

fn optional_file_bytes(path: &Path) -> Option<u64> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Some(metadata.len()),
        Ok(_) => None,
        Err(err) if err.kind() == ErrorKind::NotFound => Some(0),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_schedule_batches_small_writes_and_resets_after_reconciliation() {
        let mut schedule = CachePruneSchedule::default();
        let path = Path::new("cache.wfc");

        for _ in 0..MAX_WRITES_BETWEEN_PRUNES - 1 {
            schedule.record_success(path, Some(1));
            assert!(!schedule.immediate_prune_due());
        }
        schedule.record_success(path, Some(1));
        assert!(schedule.immediate_prune_due());

        schedule.reset();
        assert_eq!(schedule.successful_writes(), 0);
        assert_eq!(schedule.bytes_written(), 0);
        assert!(schedule.pinned_path().is_none());
    }

    #[test]
    fn prune_schedule_forces_reconciliation_at_the_byte_tolerance() {
        let mut schedule = CachePruneSchedule::default();
        let path = Path::new("large-cache.wfc");

        schedule.record_success(path, Some(MAX_BYTES_BETWEEN_PRUNES - 1));
        assert!(!schedule.immediate_prune_due());
        schedule.record_success(path, Some(1));
        assert!(schedule.immediate_prune_due());
    }

    #[test]
    fn unknown_written_size_is_conservatively_due_immediately() {
        let mut schedule = CachePruneSchedule::default();
        schedule.record_success(Path::new("unknown-size.wfc"), None);
        assert!(schedule.immediate_prune_due());
    }

    #[test]
    fn published_cache_size_includes_companion_artifacts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join("with-companions.wfc");
        fs::write(&cache_path, [0_u8]).expect("write cache file");
        fs::write(playback_sidecar_path(&cache_path), [0_u8; 7]).expect("write sidecar");
        fs::File::create(playback_descriptor_path(&cache_path))
            .expect("create descriptor")
            .set_len(MAX_BYTES_BETWEEN_PRUNES)
            .expect("size descriptor");

        let published_bytes = published_cache_bytes(&cache_path);
        assert_eq!(
            published_bytes,
            Some(MAX_BYTES_BETWEEN_PRUNES.saturating_add(8))
        );

        let mut schedule = CachePruneSchedule::default();
        schedule.record_success(&cache_path, published_bytes);
        assert!(schedule.immediate_prune_due());
    }

    #[test]
    fn startup_reconciliation_recovers_stale_cache_artifacts() {
        let config_base = tempfile::tempdir().expect("config base");
        let _base_guard =
            wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
        let cache_dir = wavecrate::app_dirs::waveform_cache_dir().expect("waveform cache dir");
        std::fs::create_dir_all(&cache_dir).expect("create waveform cache dir");
        let stale_temp = cache_dir.join("interrupted.tmp");
        std::fs::write(&stale_temp, [1_u8]).expect("write stale cache temp");

        reconcile_cache(None, 0, 0, "startup-test");

        assert!(!stale_temp.exists());
    }
}
