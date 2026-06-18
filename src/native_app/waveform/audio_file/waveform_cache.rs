use std::{path::PathBuf, sync::Arc, time::Duration};

use super::WaveformFile;
use diagnostics::log_stale_cache_entry;
use format::{CACHE_FORMAT_VERSION, CACHE_FORMAT_VERSION_V2};
use identity::CacheIdentity;
pub(in crate::native_app) use playback_load::{
    load_cached_waveform_file_for_playback, load_cached_waveform_file_summary,
};
pub(in crate::native_app) use read::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
};
use read::{read_cached_waveform_file, read_cached_waveform_file_v2};
pub(in crate::native_app) use store_queue::flush_background_waveform_cache_stores_for_shutdown;
#[cfg(test)]
pub(super) use store_queue::store_cached_waveform_file;
pub(super) use store_queue::store_cached_waveform_file_in_background;

mod diagnostics;
mod format;
mod identity;
mod playback_load;
mod prune;
mod read;
mod store_queue;
#[cfg(test)]
mod tests;
mod write;

const GIB: usize = 1024 * 1024 * 1024;
pub(super) const MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES: usize = 8 * GIB;
pub(super) const MAX_PERSISTED_WAVEFORM_CACHE_BYTES: u64 = 64 * GIB as u64;
pub(super) const BACKGROUND_STORE_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);

pub(super) fn load_cached_waveform_file(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
) -> Option<WaveformFile> {
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(cached) = read_cached_waveform_file(&path, &identity) {
        let file = cached.into_waveform_file(path.clone(), audio_bytes, identity);
        if file.is_none() {
            log_stale_cache_entry(&path, CACHE_FORMAT_VERSION);
        }
        return file;
    }
    read_cached_waveform_file_v2(&path, &identity).and_then(|cached| {
        let file = cached.into_waveform_file(path.clone(), audio_bytes, identity);
        if file.is_none() {
            log_stale_cache_entry(&path, CACHE_FORMAT_VERSION_V2);
        }
        file
    })
}
