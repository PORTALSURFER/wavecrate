use super::*;
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::Rating;

/// Side-effect policies for preview and commit focus transitions.
mod side_effects;
/// Database-backed tag/loop mutation helpers for wav entries.
mod tags;
use side_effects::SelectionSideEffects;
pub(crate) use tags::{
    set_sample_locked_for_source, set_sample_looped_for_source, set_sample_tag,
    set_sample_tag_and_locked_for_source, set_sample_tag_for_source,
};

pub(crate) fn select_wav_by_path(controller: &mut AppController, path: &Path) {
    select_wav_by_path_with_rebuild(controller, path, true);
}

/// Focus a wav path without queueing an audio/waveform load.
///
/// This is used for high-frequency browser focus navigation where selection
/// state should update immediately but loading is committed separately.
pub(crate) fn focus_wav_by_path_with_rebuild(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
) {
    select_wav_path_with_options(
        controller,
        path,
        rebuild,
        SelectionSideEffects {
            queue_audio_load: false,
            ..SelectionSideEffects::commit()
        },
    );
}

/// Preview-focus a wav path while skipping heavy commit side effects.
///
/// This path is used by wheel/arrow/high-frequency browser navigation.
pub(crate) fn focus_wav_by_path_preview_with_rebuild(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
) {
    select_wav_path_with_options(controller, path, rebuild, SelectionSideEffects::preview());
}

/// Preview-focus a wav entry by index while skipping heavy commit side effects.
pub(crate) fn focus_wav_by_index_preview_with_rebuild(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
) {
    select_wav_index_with_options(controller, index, rebuild, SelectionSideEffects::preview());
}

pub(crate) fn select_wav_by_path_with_rebuild(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
) {
    select_wav_path_with_options(controller, path, rebuild, SelectionSideEffects::commit());
}

/// Select a wav entry by index, optionally deferring browser list rebuild.
pub(crate) fn select_wav_by_index_with_rebuild(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
) {
    select_wav_index_with_options(controller, index, rebuild, SelectionSideEffects::commit());
}

/// Shared wav-path selection pipeline with optional audio load queueing.
///
/// Side effects are controlled by `side_effects` so high-frequency navigation
/// can remain lightweight while commit actions preserve full behavior.
fn select_wav_path_with_options(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    let Some(index) = controller.wav_index_for_path(path) else {
        return;
    };
    select_wav_known_index_with_options(
        controller,
        index,
        path.to_path_buf(),
        rebuild,
        side_effects,
    );
}

/// Shared wav-index selection pipeline for hot paths where entry index is known.
fn select_wav_index_with_options(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    let path = match controller.wav_entry(index) {
        Some(entry) => entry.relative_path.clone(),
        None => return,
    };
    select_wav_known_index_with_options(controller, index, path, rebuild, side_effects);
}

/// Shared selection implementation with pre-resolved absolute index and path.
fn select_wav_known_index_with_options(
    controller: &mut AppController,
    index: usize,
    path: PathBuf,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    if side_effects.commit_pending_age_update {
        controller.defer_pending_age_update_commit_if_path_changes(&path);
    }
    if controller.current_source().is_none() {
        if let Some(source_id) = controller
            .selection_state
            .ctx
            .last_selected_browsable_source
            .clone()
            .filter(|id| controller.library.sources.iter().any(|s| &s.id == id))
        {
            controller.selection_state.ctx.selected_source = Some(source_id);
            controller.refresh_sources_ui();
        } else if let Some(first) = controller.library.sources.first().cloned() {
            controller
                .selection_state
                .ctx
                .last_selected_browsable_source = Some(first.id.clone());
            controller.selection_state.ctx.selected_source = Some(first.id);
            controller.refresh_sources_ui();
        }
    }
    let path_changed = controller.sample_view.wav.selected_wav.as_deref() != Some(path.as_path());
    let active_loop_enabled = controller.ui.waveform.loop_enabled;
    let entry_looped = controller
        .wav_entries
        .entry(index)
        .map(|entry| entry.looped)
        .unwrap_or(false);
    let loop_lock_enabled = controller.ui.waveform.loop_lock_enabled;
    if path_changed {
        let _ = controller.commit_edit_selection_fades();
    }
    if path_changed {
        controller.ui.waveform.last_start_marker = None;
        if !loop_lock_enabled
            && !(controller.settings.feature_flags.autoplay_selection && active_loop_enabled)
        {
            controller.ui.waveform.loop_enabled = entry_looped;
        }
    }
    controller.sample_view.wav.selected_wav = Some(path.clone());
    controller.ui.browser.last_focused_index = Some(index);
    controller.ui.browser.last_focused_path = Some(path.clone());
    if path_changed {
        if side_effects.record_focus_history {
            controller.record_focus_history(&path);
        }
        if !side_effects.refresh_similarity_highlight {
            controller.clear_focused_similarity_highlight();
        }
    }
    let missing = controller
        .wav_entries
        .entry(index)
        .map(|entry| entry.missing)
        .unwrap_or(false);
    if missing {
        controller.show_missing_waveform_notice(&path);
        controller.set_status(
            format!("File missing: {}", path.display()),
            StatusTone::Warning,
        );
        controller.selection_state.suppress_autoplay_once = false;
        controller.clear_focused_similarity_highlight();
        if rebuild {
            controller.rebuild_browser_lists();
        }
        return;
    }
    if path_changed && side_effects.refresh_similarity_highlight {
        if let Some(source) = controller.current_source() {
            let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &path);
            controller.defer_focused_similarity_highlight_refresh(
                sample_id,
                path.clone(),
                Some(index),
            );
        } else {
            controller.clear_focused_similarity_highlight();
        }
    }
    if !side_effects.queue_audio_load {
        controller.selection_state.suppress_autoplay_once = false;
    } else if let Some(source) = controller.current_source() {
        let autoplay = controller.settings.feature_flags.autoplay_selection
            && !controller.selection_state.suppress_autoplay_once;
        controller.selection_state.suppress_autoplay_once = false;
        let selection_looped = if active_loop_enabled {
            true
        } else if path_changed && !loop_lock_enabled {
            entry_looped
        } else {
            controller.ui.waveform.loop_enabled
        };
        let pending_playback = if autoplay {
            Some(PendingPlayback {
                source_id: source.id.clone(),
                relative_path: path.clone(),
                looped: selection_looped,
                start_override: None,
            })
        } else {
            None
        };
        if let Err(err) = controller.queue_audio_load_for(
            &source,
            &path,
            AudioLoadIntent::Selection,
            pending_playback,
        ) {
            controller.set_status(err, StatusTone::Error);
        }
    } else {
        controller.selection_state.suppress_autoplay_once = false;
    }
    if rebuild {
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn select_wav_by_index(controller: &mut AppController, index: usize) {
    select_wav_by_index_with_rebuild(controller, index, true);
}

pub(crate) fn select_from_browser(controller: &mut AppController, path: &Path) {
    controller.focus_browser_context();
    select_wav_by_path(controller, path);
}

pub(crate) fn triage_flag_drop_target(controller: &AppController) -> TriageFlagColumn {
    match controller.ui.browser.filter {
        TriageFlagFilter::All | TriageFlagFilter::Untagged => TriageFlagColumn::Neutral,
        TriageFlagFilter::Keep => TriageFlagColumn::Keep,
        TriageFlagFilter::Trash => TriageFlagColumn::Trash,
    }
}

pub(crate) fn selected_tag(controller: &mut AppController) -> Option<Rating> {
    controller
        .selected_row_index()
        .and_then(|idx| controller.wav_entry(idx))
        .map(|entry| entry.tag)
}

pub(crate) fn rebuild_wav_lookup(controller: &mut AppController) {
    controller.wav_entries.lookup.clear();
    let mut entries = Vec::new();
    for (page_index, page) in controller.wav_entries.pages.iter() {
        let base = page_index * controller.wav_entries.page_size;
        for (idx, entry) in page.iter().enumerate() {
            entries.push((entry.relative_path.clone(), base + idx));
        }
    }
    for (path, index) in entries {
        controller.wav_entries.insert_lookup(path, index);
    }
}

pub(crate) fn invalidate_cached_audio_for_entry_updates(
    controller: &mut AppController,
    source_id: &SourceId,
    updates: &[(WavEntry, WavEntry)],
) {
    for (old_entry, new_entry) in updates {
        controller.invalidate_cached_audio(source_id, &old_entry.relative_path);
        controller.invalidate_cached_audio(source_id, &new_entry.relative_path);
    }
}
