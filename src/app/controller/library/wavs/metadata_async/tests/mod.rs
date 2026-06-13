use super::*;
use crate::app::controller::library::source_write_priority;
use std::path::Path;
use std::sync::{LazyLock, Mutex, MutexGuard};

mod analysis_resolution;
mod diagnostics;
mod loaded_duration;
mod operation_logging;
mod priority;
mod source_remap;

static METADATA_ASYNC_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn metadata_async_test_lock() -> MutexGuard<'static, ()> {
    METADATA_ASYNC_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
