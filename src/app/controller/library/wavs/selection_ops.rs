use super::*;
use crate::sample_sources::Rating;

/// Wav-selection cache and lookup maintenance helpers.
mod maintenance;
/// Staged focus/commit selection pipeline.
mod pipeline;
/// Side-effect policies for preview and commit focus transitions.
mod side_effects;
/// Database-backed tag/loop mutation helpers for wav entries.
mod tags;
/// Target resolution for path and index selection entrypoints.
mod target;

pub(crate) use maintenance::{invalidate_cached_audio_for_entry_updates, rebuild_wav_lookup};
use side_effects::SelectionSideEffects;
pub(crate) use tags::{
    apply_normal_tag_for_source, normal_tag_state_for_source, normal_tags_for_path,
    remove_normal_tag_for_source, set_normal_tag_for_source_batch, set_sample_locked_for_source,
    set_sample_looped_for_source, set_sample_looped_for_source_batch,
    set_sample_sound_type_for_source, set_sample_tag, set_sample_tag_and_locked_for_source,
    set_sample_tag_for_source, set_sample_user_tag_for_source,
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
    target::select_wav_path_with_options(
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
    target::select_wav_path_with_options(
        controller,
        path,
        rebuild,
        SelectionSideEffects::preview(),
    );
}

/// Preview-focus a wav entry by index while skipping heavy commit side effects.
pub(crate) fn focus_wav_by_index_preview_with_rebuild(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
) {
    target::select_wav_index_with_options(
        controller,
        index,
        rebuild,
        SelectionSideEffects::preview(),
    );
}

pub(crate) fn select_wav_by_path_with_rebuild(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
) {
    target::select_wav_path_with_options(controller, path, rebuild, SelectionSideEffects::commit());
}

/// Select a wav entry by index, optionally deferring browser list rebuild.
pub(crate) fn select_wav_by_index_with_rebuild(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
) {
    target::select_wav_index_with_options(
        controller,
        index,
        rebuild,
        SelectionSideEffects::commit(),
    );
}

pub(crate) fn select_wav_by_index(controller: &mut AppController, index: usize) {
    select_wav_by_index_with_rebuild(controller, index, true);
}

pub(crate) fn select_from_browser(controller: &mut AppController, path: &Path) {
    controller.focus_browser_context();
    select_wav_by_path(controller, path);
}

pub(crate) fn triage_flag_drop_target(controller: &AppController) -> TriageFlagColumn {
    match controller.ui.browser.search.filter {
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
