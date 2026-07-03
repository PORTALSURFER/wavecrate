use std::{fs, path::PathBuf, sync::Arc, time::Instant};

use super::{
    diagnostics::{log_slow_cache_phase, log_stale_cache_entry},
    format::CACHE_FORMAT_VERSION,
    identity::CacheIdentity,
    read::{read_cached_playback_descriptor, read_cached_waveform_file},
    store_queue::store_cached_waveform_file_in_background,
    write::{mark_cached_waveform_file_playback_ready, write_playback_descriptor_sidecar},
};
use crate::native_app::waveform::audio_file::{PersistedPlaybackDescriptor, WaveformFile};

pub(in crate::native_app) fn load_cached_waveform_file_for_playback(
    path: PathBuf,
) -> Option<WaveformFile> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(cached) = read_cached_waveform_file(&path, &identity)
        && cached.playback_cache.is_some()
    {
        let cache_path = super::identity::cache_path_for_identity(&path, &identity).ok()?;
        let _ = write_playback_descriptor_sidecar(&cache_path, &cached);
        let Some(file) = cached.into_playback_ready_waveform_file(path.clone(), identity) else {
            log_stale_cache_entry(&path, CACHE_FORMAT_VERSION);
            return None;
        };
        mark_cached_waveform_file_playback_ready(&path);
        log_slow_cache_phase(
            "browser.sample_cache.load_playback_ready",
            &path,
            started_at,
        );
        return Some(file);
    }
    if super::super::should_use_file_backed_wav_decode(&path) {
        return None;
    }

    let cached = read_cached_waveform_file(&path, &identity)?;

    let audio_bytes: Arc<[u8]> = Arc::from(fs::read(&path).ok()?);
    let Some(mut file) = cached.into_waveform_file(path.clone(), audio_bytes, identity) else {
        log_stale_cache_entry(&path, CACHE_FORMAT_VERSION);
        return None;
    };
    if file.playback_samples.is_none()
        && super::super::is_wav_path(&path)
        && let Ok(samples) = super::super::wav_decode::read_wav_playback_samples(&file.audio_bytes)
    {
        file.playback_samples = Some(Arc::from(samples));
        store_cached_waveform_file_in_background(&file);
    } else if file.playback_samples.is_some() {
        mark_cached_waveform_file_playback_ready(&path);
    }
    log_slow_cache_phase("browser.sample_cache.load_for_playback", &path, started_at);
    file.playback_samples.is_some().then_some(file)
}

pub(in crate::native_app) fn load_cached_waveform_playback_descriptor_sidecar(
    path: PathBuf,
) -> Option<PersistedPlaybackDescriptor> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    let descriptor = read_cached_playback_descriptor(&path, &identity)?;
    log_playback_descriptor_source(&path, "sidecar");
    log_slow_cache_phase(
        "browser.sample_cache.load_playback_descriptor_sidecar",
        &path,
        started_at,
    );
    Some(descriptor)
}

pub(in crate::native_app) fn load_cached_waveform_playback_descriptor(
    path: PathBuf,
) -> Option<PersistedPlaybackDescriptor> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(descriptor) = read_cached_playback_descriptor(&path, &identity) {
        log_playback_descriptor_source(&path, "sidecar");
        log_slow_cache_phase(
            "browser.sample_cache.load_playback_descriptor",
            &path,
            started_at,
        );
        return Some(descriptor);
    }
    let Some(cached) = read_cached_waveform_file(&path, &identity) else {
        return None;
    };
    if cached.playback_cache.is_none() {
        return None;
    }
    let cache_path = super::identity::cache_path_for_identity(&path, &identity).ok()?;
    let migrated_descriptor = write_playback_descriptor_sidecar(&cache_path, &cached);
    let Some(descriptor) = cached.into_playback_descriptor(path.clone(), identity) else {
        log_stale_cache_entry(&path, CACHE_FORMAT_VERSION);
        return None;
    };
    log_playback_descriptor_source(
        &path,
        if migrated_descriptor {
            "legacy_cache_migrated"
        } else {
            "legacy_cache"
        },
    );
    mark_cached_waveform_file_playback_ready(&path);
    log_slow_cache_phase(
        "browser.sample_cache.load_playback_descriptor",
        &path,
        started_at,
    );
    Some(descriptor)
}

fn log_playback_descriptor_source(path: &std::path::Path, descriptor_source: &'static str) {
    tracing::info!(
        target: "wavecrate::debug::sample_cache",
        event = "browser.sample_cache.playback_descriptor",
        descriptor_source,
        path = %path.display(),
        "Playback descriptor available"
    );
}

pub(in crate::native_app) fn load_cached_waveform_file_summary(
    path: PathBuf,
) -> Option<WaveformFile> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    let cached = read_cached_waveform_file(&path, &identity)?;
    let Some(file) = cached.into_summary_waveform_file(path.clone(), identity) else {
        log_stale_cache_entry(&path, CACHE_FORMAT_VERSION);
        return None;
    };
    log_slow_cache_phase("browser.sample_cache.load_summary", &path, started_at);
    Some(file)
}
