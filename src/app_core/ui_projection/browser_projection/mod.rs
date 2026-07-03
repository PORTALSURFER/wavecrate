//! Browser panel projection, row-window virtualization, and retained row-cache helpers.

use crate::app_core::controller::AppController;

/// Retained selection/row cache helpers for browser projection.
mod cache;
mod chrome;
mod filters;
mod frame;
mod preload;
mod row_window;
mod rows;
mod tag_sidebar;

use cache::clear_projected_selected_paths_lookup;

#[cfg(test)]
pub(super) use cache::browser_row_identity_hash;
pub(super) use cache::{
    project_cached_browser_row, refresh_projected_browser_row_cache,
    refresh_projected_selected_paths_lookup, selected_index_is_selected,
};
pub(crate) use chrome::project_browser_chrome_model;
use chrome::{browser_search_placeholder, browser_sort_label, browser_tab_label};
#[cfg(test)]
pub(crate) use frame::project_browser_panel_frame_model;
pub(crate) use frame::{
    project_browser_focused_sample_label, project_browser_model,
    project_browser_panel_frame_model_without_sidebar,
};
pub(crate) use tag_sidebar::project_browser_tag_sidebar_model;
pub(crate) type BrowserRowsProjectionInputs = rows::BrowserRowsProjectionInputs;
#[cfg(test)]
pub(crate) use preload::browser_bpm_preload_ranges;
use preload::preload_browser_window_bpms;
pub(crate) use row_window::browser_column_index;
#[cfg(test)]
pub(crate) use row_window::browser_render_window;
use row_window::{
    browser_bucket_label, clear_projected_browser_row_cache, project_browser_rows_model,
};
pub(crate) use row_window::{patch_browser_rows_state, project_browser_rows_model_into};

pub(crate) fn project_browser_rows_projection_inputs(
    controller: &AppController,
) -> BrowserRowsProjectionInputs {
    rows::project_browser_rows_projection_inputs(controller)
}
