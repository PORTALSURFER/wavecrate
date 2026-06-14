use super::*;
use rand::Rng;
#[cfg(test)]
use rand::{SeedableRng, rngs::StdRng};
use std::path::{Path, PathBuf};

mod executor;
mod history;
mod planner;

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
    history::mark_path_for_current_list(controller, source_id, relative_path);
    history::push_entry(controller, source_id.clone(), relative_path.to_path_buf());
}

pub(crate) fn mark_random_navigation_path_for_current_list(
    controller: &mut AppController,
    source_id: &SourceId,
    relative_path: &Path,
) {
    history::mark_path_for_current_list(controller, source_id, relative_path);
}

pub(crate) fn play_previous_random_sample(controller: &mut AppController) {
    match history::step_back(controller) {
        history::PreviousRandomStep::Empty => {
            controller.set_status_message(StatusMessage::RandomHistoryEmpty);
        }
        history::PreviousRandomStep::AtStart => {
            controller.set_status_message(StatusMessage::RandomHistoryStart);
        }
        history::PreviousRandomStep::Entry(entry) => {
            executor::play_history_entry(controller, entry)
        }
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
    executor::play_visible_target(controller, target, start_playback);
}

fn next_random_visible_target<R: Rng + ?Sized>(
    controller: &mut AppController,
    rng: &mut R,
) -> Option<planner::RandomVisibleTarget> {
    let Some(source_id) = controller.selection_state.ctx.selected_source.clone() else {
        controller.set_status_message(StatusMessage::SelectSourceFirst {
            tone: StatusTone::Info,
        });
        return None;
    };
    if controller.visible_browser_len() == 0 {
        controller.set_status_message(StatusMessage::NoSamplesToRandomize);
        return None;
    }

    let visible_list = planner::RandomVisibleList::from_controller(controller, source_id);
    let current_path = planner::current_path(controller);
    let available_rows =
        history::available_unplayed_rows(controller, &visible_list, current_path.as_deref());
    visible_list.choose_target(&available_rows, rng)
}

fn mark_current_random_navigation_focus(controller: &mut AppController) {
    let Some(source_id) = controller.selection_state.ctx.selected_source.clone() else {
        return;
    };
    let Some(path) = planner::current_path(controller) else {
        return;
    };
    history::mark_path_for_current_list(controller, &source_id, &path);
}
