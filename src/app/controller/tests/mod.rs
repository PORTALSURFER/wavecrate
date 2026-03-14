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
mod drag_drop_drop_targets;
mod drag_drop_folders;
mod drag_drop_waveform;
mod edit_selection_no_snap;
mod external_drop_import;
mod focus_random;
mod folders_core;
mod folders_search;
/// Map focus/preview workflow regression tests.
mod map_view;
mod missing;
mod playback_loop;
mod rating_logic;
mod recording;
mod selection_bpm_scale;
mod selection_undo;
mod transient_options;
mod trash;
/// Volume slider controller tests.
mod volume;
mod waveform;
/// Cached waveform load behavior regression tests.
mod waveform_cache_loading;
mod waveform_nav_cursor;
mod waveform_nav_render;
