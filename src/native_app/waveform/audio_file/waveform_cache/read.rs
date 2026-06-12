use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Instant,
};

use serde::de::DeserializeOwned;

use super::{
    diagnostics::log_slow_cache_phase,
    format::{
        CACHE_FORMAT_VERSION, CACHE_FORMAT_VERSION_V2, CachedWaveformFile, CachedWaveformFileV2,
    },
    identity::{
        CacheIdentity, cache_path_for_identity, cache_path_for_identity_with_version,
        playback_ready_marker_path,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CacheReadStatus {
    Hit,
    Missing,
    Corrupt,
    IoError,
}

#[derive(Debug)]
pub(super) struct CacheReadOutcome<T> {
    cache_path: PathBuf,
    status: CacheReadStatus,
    value: Option<T>,
    detail: Option<String>,
}

impl<T> CacheReadOutcome<T> {
    #[cfg(test)]
    pub(super) fn status(&self) -> CacheReadStatus {
        self.status
    }

    pub(super) fn into_hit(self) -> Option<T> {
        self.value
    }

    pub(super) fn log_if_unusable(&self, source_path: &Path, format_version: u32) {
        match self.status {
            CacheReadStatus::Hit | CacheReadStatus::Missing => {}
            CacheReadStatus::Corrupt => {
                tracing::warn!(
                    target: "wavecrate::debug::sample_cache",
                    event = "browser.sample_cache.read_corrupt",
                    source_path = %source_path.display(),
                    cache_path = %self.cache_path.display(),
                    cache_format_version = format_version,
                    detail = self.detail.as_deref().unwrap_or("deserialize failed"),
                    "Waveform cache entry is corrupt"
                );
            }
            CacheReadStatus::IoError => {
                tracing::warn!(
                    target: "wavecrate::debug::sample_cache",
                    event = "browser.sample_cache.read_io_error",
                    source_path = %source_path.display(),
                    cache_path = %self.cache_path.display(),
                    cache_format_version = format_version,
                    detail = self.detail.as_deref().unwrap_or("read failed"),
                    "Waveform cache entry could not be read"
                );
            }
        }
    }

    fn hit(cache_path: PathBuf, value: T) -> Self {
        Self {
            cache_path,
            status: CacheReadStatus::Hit,
            value: Some(value),
            detail: None,
        }
    }

    fn miss(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            status: CacheReadStatus::Missing,
            value: None,
            detail: None,
        }
    }

    fn corrupt(cache_path: PathBuf, detail: impl fmt::Display) -> Self {
        Self {
            cache_path,
            status: CacheReadStatus::Corrupt,
            value: None,
            detail: Some(detail.to_string()),
        }
    }

    fn io_error(cache_path: PathBuf, detail: impl fmt::Display) -> Self {
        Self {
            cache_path,
            status: CacheReadStatus::IoError,
            value: None,
            detail: Some(detail.to_string()),
        }
    }
}

pub(in crate::native_app) fn cached_waveform_file_exists(path: &Path) -> bool {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return false;
    };
    cache_path_for_identity(path, &identity).is_ok_and(|path| path.is_file())
        || cache_path_for_identity_with_version(path, &identity, CACHE_FORMAT_VERSION_V2)
            .is_ok_and(|path| path.is_file())
}

pub(in crate::native_app) fn cached_waveform_file_playback_ready_exists(path: &Path) -> bool {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return false;
    };
    let Ok(cache_path) = cache_path_for_identity(path, &identity) else {
        return false;
    };
    if cache_path.is_file()
        && playback_ready_marker_path(&cache_path).is_file()
        && read_cached_waveform_file(path, &identity)
            .and_then(|cached| cached.playback_cache_file(&cache_path))
            .is_some()
    {
        return true;
    }
    cache_path_for_identity_with_version(path, &identity, CACHE_FORMAT_VERSION_V2).is_ok_and(
        |v2_cache_path| {
            v2_cache_path.is_file() && playback_ready_marker_path(&v2_cache_path).is_file()
        },
    )
}

pub(super) fn read_cached_waveform_file(
    path: &Path,
    identity: &CacheIdentity,
) -> Option<CachedWaveformFile> {
    let outcome = read_cached_waveform_file_outcome(path, identity);
    outcome.log_if_unusable(path, CACHE_FORMAT_VERSION);
    outcome.into_hit()
}

pub(super) fn read_cached_waveform_file_outcome(
    path: &Path,
    identity: &CacheIdentity,
) -> CacheReadOutcome<CachedWaveformFile> {
    let cache_path = match cache_path_for_identity(path, identity) {
        Ok(cache_path) => cache_path,
        Err(err) => return CacheReadOutcome::io_error(path.to_path_buf(), err),
    };
    read_cache_file(
        path,
        cache_path,
        "browser.sample_cache.metadata_read",
        "browser.sample_cache.metadata_deserialize",
    )
}

pub(super) fn read_cached_waveform_file_v2(
    path: &Path,
    identity: &CacheIdentity,
) -> Option<CachedWaveformFileV2> {
    let outcome = read_cached_waveform_file_v2_outcome(path, identity);
    outcome.log_if_unusable(path, CACHE_FORMAT_VERSION_V2);
    outcome.into_hit()
}

pub(super) fn read_cached_waveform_file_v2_outcome(
    path: &Path,
    identity: &CacheIdentity,
) -> CacheReadOutcome<CachedWaveformFileV2> {
    let cache_path =
        match cache_path_for_identity_with_version(path, identity, CACHE_FORMAT_VERSION_V2) {
            Ok(cache_path) => cache_path,
            Err(err) => return CacheReadOutcome::io_error(path.to_path_buf(), err),
        };
    read_cached_waveform_file_v2_at(path, cache_path)
}

pub(super) fn read_cached_waveform_file_v2_at(
    source_path: &Path,
    cache_path: PathBuf,
) -> CacheReadOutcome<CachedWaveformFileV2> {
    read_cache_file(
        source_path,
        cache_path,
        "browser.sample_cache.v2_read",
        "browser.sample_cache.v2_deserialize",
    )
}

fn read_cache_file<T>(
    source_path: &Path,
    cache_path: PathBuf,
    read_event: &'static str,
    deserialize_event: &'static str,
) -> CacheReadOutcome<T>
where
    T: DeserializeOwned,
{
    let read_started_at = Instant::now();
    let bytes = match fs::read(&cache_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return CacheReadOutcome::miss(cache_path);
        }
        Err(err) => {
            return CacheReadOutcome::io_error(cache_path, err);
        }
    };
    log_slow_cache_phase(read_event, source_path, read_started_at);
    let deserialize_started_at = Instant::now();
    let cached = match bincode::deserialize(&bytes) {
        Ok(cached) => cached,
        Err(err) => return CacheReadOutcome::corrupt(cache_path, err),
    };
    log_slow_cache_phase(deserialize_event, source_path, deserialize_started_at);
    CacheReadOutcome::hit(cache_path, cached)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::waveform::audio_file::waveform_cache::identity::cache_path_for_identity;

    #[test]
    fn cache_read_outcome_classifies_missing_corrupt_and_io_failure_entries() {
        let dir = tempfile::tempdir().expect("tempdir");

        let (missing_path, missing_identity, _) = cache_read_test_paths(dir.path(), "missing.wav");
        let missing = read_cached_waveform_file_outcome(&missing_path, &missing_identity);
        assert_eq!(missing.status(), CacheReadStatus::Missing);
        assert!(missing.into_hit().is_none());

        let (corrupt_path, corrupt_identity, corrupt_cache_path) =
            cache_read_test_paths(dir.path(), "corrupt.wav");
        fs::write(&corrupt_cache_path, b"not a bincode waveform cache")
            .expect("write corrupt cache");
        let corrupt = read_cached_waveform_file_outcome(&corrupt_path, &corrupt_identity);
        assert_eq!(corrupt.status(), CacheReadStatus::Corrupt);
        assert!(corrupt.into_hit().is_none());

        let (io_path, io_identity, io_cache_path) =
            cache_read_test_paths(dir.path(), "directory-cache.wav");
        let _ = fs::remove_file(&io_cache_path);
        fs::create_dir_all(&io_cache_path).expect("cache path as directory");
        let io_failure = read_cached_waveform_file_outcome(&io_path, &io_identity);
        assert_eq!(io_failure.status(), CacheReadStatus::IoError);
        assert!(io_failure.into_hit().is_none());
    }

    fn cache_read_test_paths(dir: &Path, file_name: &str) -> (PathBuf, CacheIdentity, PathBuf) {
        let path = dir.join(file_name);
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let identity = CacheIdentity::for_path(&path).expect("identity");
        let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");
        fs::create_dir_all(cache_path.parent().expect("cache dir")).expect("cache dir");
        (path, identity, cache_path)
    }
}
