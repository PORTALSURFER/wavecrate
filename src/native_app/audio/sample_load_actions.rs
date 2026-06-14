use radiant::prelude as ui;
use std::time::Duration;

pub(in crate::native_app) const UNCACHED_SAMPLE_LOAD_DEBOUNCE: Duration = Duration::from_millis(90);
pub(in crate::native_app) const KEYBOARD_SAMPLE_LOAD_DEBOUNCE: Duration =
    UNCACHED_SAMPLE_LOAD_DEBOUNCE;

pub(in crate::native_app) use types::{NormalizedWaveformReload, WaveformPlaybackResume};

mod cache;
mod cache_start;
mod completion;
mod deferred_drop;
mod diagnostics;
mod entrypoints;
mod plan;
mod playback_state;
mod reload;
mod types;
mod worker;

#[cfg(test)]
pub(in crate::native_app) use cache::{
    ACTIVE_FOLDER_CACHE_WARM_MAX_PENDING_FILES, active_folder_cache_warm_priority,
    warm_active_folder_waveform_cache, warm_persisted_waveform_cache,
};

pub(super) use diagnostics::{
    log_loaded_sample_metadata, log_sample_load_timing, log_slow_sample_load_phase,
};

pub(in crate::native_app) fn foreground_sample_load_priority() -> ui::TaskPriority {
    ui::TaskPriority::Interactive
}

pub(in crate::native_app) fn sample_resource_key(path: &str) -> ui::ResourceKey {
    ui::ResourceKey::scoped("sample", path)
}

pub(in crate::native_app) fn waveform_cache_warm_resource_key() -> ui::ResourceKey {
    ui::ResourceKey::scoped("waveform_cache", "warm")
}

pub(in crate::native_app) fn active_folder_cache_warm_resource_key(
    folder_id: &str,
) -> ui::ResourceKey {
    ui::ResourceKey::scoped("active_folder_cache_warm", folder_id)
}
