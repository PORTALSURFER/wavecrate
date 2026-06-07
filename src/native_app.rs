//! Default Wavecrate native application built on Radiant's current public API.

mod app_scope;
mod audio_engine;
mod audio_settings;
mod context_menu;
mod context_menu_actions;
mod drag_drop_actions;
mod file_actions;
mod folder_browser;
mod folder_browser_actions;
mod folder_browser_rename_actions;
mod folder_scan_actions;
mod launch;
mod layout;
mod lifecycle;
mod message_dispatch;
mod metadata_tag_metrics;
mod metadata_tags;
mod native_file_drop_actions;
mod normalization_actions;
mod playback;
mod sample_browser_view;
mod sample_collections;
mod sample_load_actions;
mod sample_ratings;
mod selected_file_actions;
mod shortcuts;
mod source_watcher;
mod state;
mod status_bar;
#[cfg(test)]
mod test_support;
mod toolbar;
mod transaction_history;
mod trash_actions;
mod waveform;
mod waveform_panel;

pub(crate) use launch::run;

#[cfg(test)]
mod tests;
