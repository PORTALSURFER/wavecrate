use super::*;
use rand::Rng;
use rand::seq::IteratorRandom;
#[cfg(test)]
use rand::{SeedableRng, rngs::StdRng};
use std::path::{Path, PathBuf};

/// Resolved random-navigation target chosen from the visible browser rows.
struct RandomVisibleTarget {
    /// Source that owns the chosen browser row.
    source_id: SourceId,
    /// Visible browser row selected for the next random jump.
    visible_row: usize,
    /// Source-relative path for the chosen sample.
    path: PathBuf,
}

pub(crate) fn play_random_visible_sample(controller: &mut AppController) {
    let mut rng = rand::rng();
    play_random_visible_sample_internal(controller, &mut rng, super::SHOULD_PLAY_RANDOM_SAMPLE);
}

#[cfg(test)]
pub(crate) fn play_random_visible_sample_with_seed(controller: &mut AppController, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    play_random_visible_sample_internal(controller, &mut rng, false);
}

pub(crate) fn focus_random_visible_sample(controller: &mut AppController) {
    let mut rng = rand::rng();
    play_random_visible_sample_internal(controller, &mut rng, false);
}

/// Return the next random visible sample path without changing browser focus.
pub(crate) fn next_random_visible_sample_path(controller: &mut AppController) -> Option<PathBuf> {
    let mut rng = rand::rng();
    next_random_visible_target(controller, &mut rng).map(|target| target.path)
}

/// Record one path as the newest random-navigation destination.
pub(crate) fn record_random_navigation_target_for_source(
    controller: &mut AppController,
    source_id: &SourceId,
    relative_path: &Path,
) {
    controller
        .history
        .random_history
        .mark_played(source_id, relative_path);
    push_random_history(controller, source_id.clone(), relative_path.to_path_buf());
}

pub(crate) fn play_previous_random_sample(controller: &mut AppController) {
    if controller.history.random_history.entries.is_empty() {
        controller.set_status_message(StatusMessage::RandomHistoryEmpty);
        return;
    }
    let current = controller.history.random_history.cursor.unwrap_or_else(|| {
        controller
            .history
            .random_history
            .entries
            .len()
            .saturating_sub(1)
    });
    if current == 0 {
        controller.history.random_history.cursor = Some(0);
        controller.set_status_message(StatusMessage::RandomHistoryStart);
        return;
    }
    let target = current - 1;
    controller.history.random_history.cursor = Some(target);
    if let Some(entry) = controller
        .history
        .random_history
        .entries
        .get(target)
        .cloned()
    {
        play_random_history_entry(controller, entry);
    }
}

pub(crate) fn toggle_random_navigation_mode(controller: &mut AppController) {
    controller.ui.browser.search.random_navigation_mode =
        !controller.ui.browser.search.random_navigation_mode;
    controller.mark_browser_search_projection_revision_dirty();
    if controller.ui.browser.search.random_navigation_mode {
        mark_current_random_navigation_focus(controller);
        controller.set_status_message(StatusMessage::custom(
            "Random navigation on: Up/Down jump to random samples",
            StatusTone::Info,
        ));
    } else {
        controller.set_status_message(StatusMessage::RandomNavOff);
    }
}

pub(crate) fn random_navigation_mode_enabled(controller: &AppController) -> bool {
    controller.ui.browser.search.random_navigation_mode
}

fn play_random_visible_sample_internal<R: Rng + ?Sized>(
    controller: &mut AppController,
    rng: &mut R,
    start_playback: bool,
) {
    let Some(target) = next_random_visible_target(controller, rng) else {
        return;
    };
    record_random_navigation_target_for_source(controller, &target.source_id, &target.path);
    controller.focus_browser_row_only(target.visible_row);
    if start_playback
        && let Err(err) = controller.play_audio(controller.ui.waveform.loop_enabled, None)
    {
        controller.set_status(err, StatusTone::Error);
    }
}

fn next_random_visible_target<R: Rng + ?Sized>(
    controller: &mut AppController,
    rng: &mut R,
) -> Option<RandomVisibleTarget> {
    let Some(source_id) = controller.selection_state.ctx.selected_source.clone() else {
        controller.set_status_message(StatusMessage::SelectSourceFirst {
            tone: StatusTone::Info,
        });
        return None;
    };
    let total = controller.visible_browser_len();
    if total == 0 {
        controller.set_status_message(StatusMessage::NoSamplesToRandomize);
        return None;
    }

    let current_path = current_random_navigation_path(controller);
    let mut available_rows =
        available_random_visible_rows(controller, &source_id, current_path.as_deref());

    if available_rows.is_empty() {
        controller
            .history
            .random_history
            .reset_played_for_source(&source_id);
        available_rows =
            available_random_visible_rows(controller, &source_id, current_path.as_deref());
    }

    let &visible_row = available_rows.iter().choose(rng)?;
    random_visible_target_for_row(controller, &source_id, visible_row)
}

fn available_random_visible_rows(
    controller: &mut AppController,
    source_id: &SourceId,
    current_path: Option<&Path>,
) -> Vec<usize> {
    let total = controller.visible_browser_len();
    let exclude_current = current_path.is_some() && total > 1;
    let mut rows = Vec::new();
    for row in 0..total {
        let Some(entry_index) = controller.visible_browser_index(row) else {
            continue;
        };
        let Some(path) = controller
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            continue;
        };
        if controller
            .history
            .random_history
            .has_played(source_id, &path)
        {
            continue;
        }
        if exclude_current && current_path.is_some_and(|selected| selected == path.as_path()) {
            continue;
        }
        rows.push(row);
    }
    rows
}

fn random_visible_target_for_row(
    controller: &mut AppController,
    source_id: &SourceId,
    visible_row: usize,
) -> Option<RandomVisibleTarget> {
    let entry_index = controller.visible_browser_index(visible_row)?;
    let path = controller
        .wav_entry(entry_index)
        .map(|entry| entry.relative_path.clone())?;
    Some(RandomVisibleTarget {
        source_id: source_id.clone(),
        visible_row,
        path,
    })
}

fn current_random_navigation_path(controller: &AppController) -> Option<PathBuf> {
    controller
        .sample_view
        .wav
        .selected_wav
        .clone()
        .or_else(|| controller.ui.browser.selection.last_focused_path.clone())
}

fn mark_current_random_navigation_focus(controller: &mut AppController) {
    let Some(source_id) = controller.selection_state.ctx.selected_source.clone() else {
        return;
    };
    let Some(path) = current_random_navigation_path(controller) else {
        return;
    };
    controller
        .history
        .random_history
        .mark_played(&source_id, &path);
}

fn push_random_history(
    controller: &mut AppController,
    source_id: SourceId,
    relative_path: PathBuf,
) {
    if let Some(cursor) = controller.history.random_history.cursor
        && cursor + 1 < controller.history.random_history.entries.len()
    {
        controller
            .history
            .random_history
            .entries
            .truncate(cursor + 1);
    }
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id,
            relative_path,
        });
    if controller.history.random_history.entries.len() > RANDOM_HISTORY_LIMIT {
        controller.history.random_history.entries.pop_front();
        if let Some(cursor) = controller.history.random_history.cursor {
            controller.history.random_history.cursor = Some(cursor.saturating_sub(1));
        }
    }
    controller.history.random_history.cursor = Some(
        controller
            .history
            .random_history
            .entries
            .len()
            .saturating_sub(1),
    );
}

fn play_random_history_entry(controller: &mut AppController, entry: RandomHistoryEntry) {
    if controller.selection_state.ctx.selected_source.as_ref() != Some(&entry.source_id) {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: entry.source_id.clone(),
                relative_path: entry.relative_path.clone(),
                looped: controller.ui.waveform.loop_enabled,
                start_override: None,
                force_loaded_audio: false,
            }));
        controller
            .runtime
            .jobs
            .set_pending_select_path(Some(entry.relative_path.clone()));
        controller.select_source_internal(Some(entry.source_id), Some(entry.relative_path));
        return;
    }
    if let Some(row) = controller.visible_row_for_path(&entry.relative_path) {
        controller.focus_browser_row_only(row);
    } else {
        controller.select_wav_by_path(&entry.relative_path);
    }
    if let Err(err) = controller.play_audio(controller.ui.waveform.loop_enabled, None) {
        controller.set_status(err, StatusTone::Error);
    }
}
