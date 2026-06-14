use super::super::*;
use super::planner::{RandomVisibleList, RandomVisibleRow};
use crate::app::controller::RANDOM_HISTORY_LIMIT;
use crate::app::controller::state::history::RandomHistoryEntry;
use std::path::{Path, PathBuf};

pub(super) enum PreviousRandomStep {
    Empty,
    AtStart,
    Entry(RandomHistoryEntry),
}

pub(super) fn available_unplayed_rows<'a>(
    controller: &mut AppController,
    visible_list: &'a RandomVisibleList,
    current_path: Option<&Path>,
) -> Vec<&'a RandomVisibleRow> {
    controller
        .history
        .random_history
        .ensure_current_list(&visible_list.source_id, visible_list.fingerprint);

    let mut rows = visible_list.available_rows(
        |path| {
            controller.history.random_history.has_played_in_list(
                &visible_list.source_id,
                visible_list.fingerprint,
                path,
            )
        },
        current_path,
    );

    if rows.is_empty() {
        controller
            .history
            .random_history
            .reset_played_for_list(&visible_list.source_id, visible_list.fingerprint);
        rows = visible_list.available_rows(|_| false, current_path);
    }

    rows
}

pub(super) fn mark_path_for_current_list(
    controller: &mut AppController,
    source_id: &SourceId,
    relative_path: &Path,
) {
    let visible_list = RandomVisibleList::from_controller(controller, source_id.clone());
    controller.history.random_history.mark_played_in_list(
        source_id,
        visible_list.fingerprint,
        relative_path,
    );
}

pub(super) fn push_entry(
    controller: &mut AppController,
    source_id: SourceId,
    relative_path: PathBuf,
) {
    truncate_forward_history(controller);
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id,
            relative_path,
        });
    trim_history_to_limit(controller);
    point_cursor_at_newest_entry(controller);
}

pub(super) fn step_back(controller: &mut AppController) -> PreviousRandomStep {
    if controller.history.random_history.entries.is_empty() {
        return PreviousRandomStep::Empty;
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
        return PreviousRandomStep::AtStart;
    }
    let target = current - 1;
    controller.history.random_history.cursor = Some(target);
    controller
        .history
        .random_history
        .entries
        .get(target)
        .cloned()
        .map_or(PreviousRandomStep::AtStart, PreviousRandomStep::Entry)
}

fn truncate_forward_history(controller: &mut AppController) {
    if let Some(cursor) = controller.history.random_history.cursor
        && cursor + 1 < controller.history.random_history.entries.len()
    {
        controller
            .history
            .random_history
            .entries
            .truncate(cursor + 1);
    }
}

fn trim_history_to_limit(controller: &mut AppController) {
    if controller.history.random_history.entries.len() <= RANDOM_HISTORY_LIMIT {
        return;
    }
    controller.history.random_history.entries.pop_front();
    if let Some(cursor) = controller.history.random_history.cursor {
        controller.history.random_history.cursor = Some(cursor.saturating_sub(1));
    }
}

fn point_cursor_at_newest_entry(controller: &mut AppController) {
    controller.history.random_history.cursor = Some(
        controller
            .history
            .random_history
            .entries
            .len()
            .saturating_sub(1),
    );
}
