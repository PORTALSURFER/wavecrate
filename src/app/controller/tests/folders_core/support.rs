pub(super) use super::super::super::test_support::{
    dummy_controller, sample_entry, write_test_wav,
};
pub(super) use super::super::common::visible_indices;
pub(super) use crate::app::controller::AppController;
pub(super) use crate::app::controller::library::source_folders::delete_recovery;
pub(super) use crate::app::state::FocusContext;
pub(super) use std::path::{Path, PathBuf};

pub(super) fn visible_paths(controller: &mut AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}
