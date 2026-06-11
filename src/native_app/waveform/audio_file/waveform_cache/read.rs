use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use super::{
    CACHE_FORMAT_VERSION_V2,
    format::{CachedWaveformFile, CachedWaveformFileV2},
    identity::{
        CacheIdentity, cache_path_for_identity, cache_path_for_identity_with_version,
        playback_ready_marker_path,
    },
    log_slow_cache_phase,
};

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
    let cache_path = cache_path_for_identity(path, identity).ok()?;
    let read_started_at = Instant::now();
    let bytes = fs::read(&cache_path).ok()?;
    log_slow_cache_phase("browser.sample_cache.metadata_read", path, read_started_at);
    let deserialize_started_at = Instant::now();
    let cached: CachedWaveformFile = bincode::deserialize(&bytes).ok()?;
    log_slow_cache_phase(
        "browser.sample_cache.metadata_deserialize",
        path,
        deserialize_started_at,
    );
    Some(cached)
}

pub(super) fn read_cached_waveform_file_v2(
    path: &Path,
    identity: &CacheIdentity,
) -> Option<CachedWaveformFileV2> {
    let cache_path =
        cache_path_for_identity_with_version(path, identity, CACHE_FORMAT_VERSION_V2).ok()?;
    read_cached_waveform_file_v2_at(path, cache_path)
}

pub(super) fn read_cached_waveform_file_v2_at(
    source_path: &Path,
    cache_path: PathBuf,
) -> Option<CachedWaveformFileV2> {
    let read_started_at = Instant::now();
    let bytes = fs::read(cache_path).ok()?;
    log_slow_cache_phase("browser.sample_cache.v2_read", source_path, read_started_at);
    let deserialize_started_at = Instant::now();
    let cached: CachedWaveformFileV2 = bincode::deserialize(&bytes).ok()?;
    log_slow_cache_phase(
        "browser.sample_cache.v2_deserialize",
        source_path,
        deserialize_started_at,
    );
    Some(cached)
}
