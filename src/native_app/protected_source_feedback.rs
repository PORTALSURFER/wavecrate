use std::path::Path;

use crate::native_app::app::NativeAppState;

pub(in crate::native_app) const PROTECTED_SOURCE_BLOCKED_STATUS: &str =
    "Protected source cannot be modified";

impl NativeAppState {
    pub(in crate::native_app) fn flash_protected_source_block_if_error(
        &mut self,
        error: &str,
        path: &Path,
    ) -> bool {
        if !self.protected_source_block_error_for_path(error, path) {
            return false;
        }
        self.library
            .folder_browser
            .flash_protected_source_error_paths([path]);
        if self.waveform.current.path() == path {
            self.waveform.current.flash_protected_source_error();
        }
        true
    }

    pub(in crate::native_app) fn protected_source_status_or_error(
        &self,
        error: &str,
        path: &Path,
    ) -> String {
        if self.protected_source_block_error_for_path(error, path) {
            PROTECTED_SOURCE_BLOCKED_STATUS.to_string()
        } else {
            error.to_string()
        }
    }

    fn protected_source_block_error_for_path(&self, error: &str, path: &Path) -> bool {
        if protected_source_block_error(error) {
            return true;
        }
        self.library
            .folder_browser
            .path_is_in_protected_source(path)
            && error.starts_with("Set a Primary source before")
    }
}

fn protected_source_block_error(error: &str) -> bool {
    error.contains("This source is protected")
        || error.contains("Protected source cannot be modified")
}
