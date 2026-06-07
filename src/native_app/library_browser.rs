//! Wavecrate library browser modules for sources, folders, sample rows, and file workflows.

pub(in crate::native_app) const DRAG_PREVIEW_MAX_WIDTH: f32 = 280.0;
pub(in crate::native_app) const DRAG_PREVIEW_HEIGHT: f32 = 20.0;

pub(in crate::native_app) mod context_menu;
pub(in crate::native_app) mod drag_drop_actions;
pub(in crate::native_app) mod file_actions;
pub(in crate::native_app) mod folder_browser;
pub(in crate::native_app) mod folder_browser_actions;
pub(in crate::native_app) mod folder_browser_rename_actions;
pub(in crate::native_app) mod folder_scan_actions;
pub(in crate::native_app) mod native_file_drop_actions;
pub(in crate::native_app) mod sample_browser_view;
pub(in crate::native_app) mod sample_collections;
pub(in crate::native_app) mod sample_ratings;
pub(in crate::native_app) mod selected_file_actions;
pub(in crate::native_app) mod source_watcher;
pub(in crate::native_app) mod trash_actions;
