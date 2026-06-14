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
pub(in crate::native_app) const WAVEFORM_CACHE_INDICATOR_REFRESH_MAX_FILES: usize = 192;
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_DELAY: Duration =
    Duration::from_millis(750);
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_MAX_PENDING_FILES: usize = 192;
pub(in crate::native_app) const ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES: usize = 1;

pub(in crate::native_app) fn active_folder_cache_warm_priority() -> ui::TaskPriority {
    ui::TaskPriority::Idle
}

#[cfg(test)]
pub(in crate::native_app) use workers::{
    warm_active_folder_waveform_cache, warm_persisted_waveform_cache,
};
