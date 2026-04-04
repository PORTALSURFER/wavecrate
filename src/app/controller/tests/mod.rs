#![allow(clippy::cmp_owned, clippy::iter_cloned_collect)]

mod audio_action_playback;
mod browser_actions;
/// Runtime-default async browser-search controller coverage.
mod browser_async;
mod browser_core;
/// Browser-row inline metadata regression tests.
mod browser_inline_tags;
mod browser_selection;
mod common;
/// Compare-anchor regression coverage for browser-relative selection anchoring.
mod compare_anchor;
mod drag_drop_browser;
mod drag_drop_drop_targets;
mod drag_drop_folders;
mod drag_drop_sources;
mod drag_drop_waveform;
mod edit_selection_no_snap;
mod external_drop_import;
mod focus_random;
/// Async folder-tree projection coverage for hot folder interactions.
mod folder_async;
mod folders_core;
mod folders_search;
mod history_transactions;
/// Map focus/preview workflow regression tests.
mod map_view;
mod missing;
mod playback_loop;
mod rating_logic;
mod recording;
mod selection_bpm_scale;
mod selection_undo;
/// Async source-hydration coverage for source switching and pane assignment.
mod source_async;
/// Startup audio-probe deferral coverage.
mod startup_audio;
mod transient_options;
mod trash;
mod undo_file_ops;
/// Volume slider controller tests.
mod volume;
mod waveform;
/// Cached waveform load behavior regression tests.
mod waveform_cache_loading;
mod waveform_nav_cursor;
mod waveform_nav_render;
