use radiant::prelude as ui;
use std::time::Duration;

mod active_folder_warm;
mod eviction;
mod indicator_refresh;
mod logging;
mod memory;
mod persisted_warm;
mod workers;

pub(in crate::native_app) const WAVEFORM_CACHE_WARM_BATCH_MAX_FILES: usize = 1;
pub(in crate::native_app) const WAVEFORM_CACHE_INDICATOR_REFRESH_MAX_FILES: usize = 64;
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_HYDRATE_MAX_FILES: usize = 32;
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_INITIAL_DELAY: Duration =
    Duration::from_millis(750);
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_CONTINUATION_DELAY: Duration =
    Duration::from_millis(350);
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_LIGHT_CONTINUATION_DELAY: Duration =
    Duration::from_millis(75);
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_SCAN_MAX_FILES: usize = 128;

pub(in crate::native_app) fn active_folder_cache_warm_priority() -> ui::TaskPriority {
    ui::TaskPriority::Idle
}

#[cfg(test)]
pub(in crate::native_app) use workers::{
    plan_active_folder_waveform_cache_warm, warm_active_folder_waveform_cache,
    warm_persisted_waveform_cache,
};
