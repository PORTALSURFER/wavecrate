use std::{fs, path::PathBuf, sync::Arc, time::Instant};

use super::{
    diagnostics::{log_slow_cache_phase, log_stale_cache_entry},
    format::{CACHE_FORMAT_VERSION, CACHE_FORMAT_VERSION_V2},
    identity::CacheIdentity,
    read::{read_cached_waveform_file, read_cached_waveform_file_v2},
    store_queue::store_cached_waveform_file_in_background,
    write::mark_cached_waveform_file_playback_ready,
};
use crate::native_app::waveform::audio_file::WaveformFile;

pub(in crate::native_app) fn load_cached_waveform_file_for_playback(
    path: PathBuf,
) -> Option<WaveformFile> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(cached) = read_cached_waveform_file(&path, &identity)
        && cached.playback_cache.is_some()
    {
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
    if let Some(cached_v2) = read_cached_waveform_file_v2(&path, &identity)
        && cached_v2.playback_samples.is_some()
    {
        let Some(file) = cached_v2.into_playback_ready_waveform_file(path.clone(), identity) else {
            log_stale_cache_entry(&path, CACHE_FORMAT_VERSION_V2);
            return None;
        };
        log_slow_cache_phase(
            "browser.sample_cache.load_v2_playback_ready",
            &path,
            started_at,
        );
        store_cached_waveform_file_in_background(&file);
        return Some(file);
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
