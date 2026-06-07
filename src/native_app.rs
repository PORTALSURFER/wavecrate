//! Default Wavecrate native application built on Radiant's current public API.

mod app_scope;
mod audio;
mod browser;
mod chrome;
mod context_menu;
mod metadata_tag_metrics;
mod metadata_tags;
mod shell;
mod state;
#[cfg(test)]
mod test_support;
mod transaction_history;
mod waveform;
mod workflows;

#[allow(unused_imports)]
pub(in crate::native_app) use audio::{
    audio_engine, audio_settings, normalization_actions, playback, sample_load_actions,
};
#[allow(unused_imports)]
pub(in crate::native_app) use browser::{
    drag_drop_actions, file_actions, folder_browser, folder_browser_actions,
    folder_browser_rename_actions, folder_scan_actions, native_file_drop_actions,
    sample_browser_view, sample_collections, sample_ratings, selected_file_actions, source_watcher,
    trash_actions,
};
#[allow(unused_imports)]
pub(in crate::native_app) use chrome::{layout, status_bar, toolbar, waveform_panel};
#[allow(unused_imports)]
pub(in crate::native_app) use shell::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_WINDOW_TITLE, debug_layout_requested,
    emit_gui_action, lifecycle, message_dispatch, shortcuts,
};
#[allow(unused_imports)]
pub(in crate::native_app) use workflows::context_menu_actions;

pub(crate) use shell::run;

#[cfg(test)]
mod tests;
