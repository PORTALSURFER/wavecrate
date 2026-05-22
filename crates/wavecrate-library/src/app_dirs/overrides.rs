//! Scoped override guards for application-directory resolution.

use std::path::PathBuf;

use super::{APP_ROOT_OVERRIDE, SCOPED_APP_ROOT_OVERRIDE, TEST_CONFIG_OVERRIDE};

/// Guard that sets a temporary config base path for tests and restores the prior value.
pub struct ConfigBaseGuard {
    previous: Option<PathBuf>,
    previous_scoped_root: Option<PathBuf>,
    previous_root: Option<PathBuf>,
}

impl ConfigBaseGuard {
    /// Override the config base directory for the lifetime of the guard.
    pub fn set(path: PathBuf) -> Self {
        let previous = TEST_CONFIG_OVERRIDE.with(|override_path| {
            let mut slot = override_path.borrow_mut();
            let prev = slot.clone();
            *slot = Some(path);
            prev
        });
        let previous_scoped_root = SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            let mut slot = override_path.borrow_mut();
            let prev = slot.clone();
            *slot = None;
            prev
        });
        let mut root_guard = APP_ROOT_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_root = root_guard.clone();
        *root_guard = None;
        Self {
            previous,
            previous_root,
            previous_scoped_root,
        }
    }
}

impl Drop for ConfigBaseGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        TEST_CONFIG_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = previous;
        });
        let previous_scoped_root = self.previous_scoped_root.take();
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = previous_scoped_root;
        });
        let mut root_guard = APP_ROOT_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *root_guard = self.previous_root.take();
    }
}
