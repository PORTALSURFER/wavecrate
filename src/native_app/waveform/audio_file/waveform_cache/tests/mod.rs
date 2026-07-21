use super::{
    identity::{
        CacheIdentity, cache_path_for_identity, playback_descriptor_path,
        playback_ready_marker_path, playback_sidecar_path, source_warm_marker_path,
    },
    prune::prune_waveform_cache_dir,
    read::{CacheReadStatus, read_cached_waveform_file_outcome},
    store_queue::{
        BackgroundStoreQueue, CachePruneSchedule, CachedWaveformStoreJob, StoreEnqueueOutcome,
    },
    write::{
        MarkerUpdateOutcome, PlaybackSidecarOutcome, StoreWriteOutcome, playback_sample_bytes,
        store_cached_waveform_file_now, update_playback_ready_marker, write_playback_sidecar,
        write_playback_sidecar_outcome,
    },
    *,
};
use crate::native_app::waveform::audio_file::waveform_file_from_mono_samples;
use std::{
    fs,
    path::Path,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

mod failures;
mod format_payload;
mod identity_staleness;
mod prune_behavior;
mod remap_behavior;
mod store_queue_behavior;

static WAVEFORM_CACHE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

fn waveform_cache_test_guard() -> MutexGuard<'static, ()> {
    WAVEFORM_CACHE_TEST_LOCK
        .lock()
        .expect("waveform cache test lock")
}

fn set_file_modified_seconds(path: &Path, seconds: i64) {
    let time = filetime::FileTime::from_unix_time(seconds, 0);
    filetime::set_file_mtime(path, time).expect("set file mtime");
}
