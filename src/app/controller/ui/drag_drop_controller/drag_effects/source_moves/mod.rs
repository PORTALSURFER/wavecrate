//! Cross-source sample move handling for drag/drop.
//!
//! The source-move pipeline is split into:
//! - `plan`: request collection and worker kickoff
//! - `apply_result`: controller-side cache/UI application
//! - `registration`: shared DB registration helpers reused by other drag effects
//! - `worker`: background execution with explicit transaction stages

mod apply_result;
mod plan;
mod registration;
mod worker;

pub(super) use registration::MovedSampleRegistration;

#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
static SOURCE_MOVE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
pub(super) fn source_move_test_guard() -> MutexGuard<'static, ()> {
    SOURCE_MOVE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("source-move test lock poisoned")
}
