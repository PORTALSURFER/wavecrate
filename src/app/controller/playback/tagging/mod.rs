use super::*;

mod shared;

use self::shared::{
    RatingUndoState, TaggingSelection, prepare_tagging_selection, push_rating_undo_entry,
    record_applied_update,
};

fn should_advance_after_rating(
    controller: &mut AppController,
    primary_row: usize,
    refocus_path: Option<&Path>,
    changed: bool,
) -> bool {
    changed
        && controller.settings.controls.advance_after_rating
        && controller.ui.browser.selection.selected_visible == Some(primary_row)
        && refocus_path.and_then(|path| controller.visible_row_for_path(path)) == Some(primary_row)
}

/// Advance rating focus or commit the filtered replacement row after a rating change.
///
/// When the rated sample remains visible, normal auto-advance moves to the next
/// visible browser row. When the active filter removes the rated sample, the
/// browser already refocuses the replacement row in the same visible position;
/// in that case this helper commits that replacement so waveform/audio loading
/// follows the filtered list instead of skipping past it.
fn advance_or_commit_after_rating(
    controller: &mut AppController,
    primary_row: usize,
    refocus_path: Option<&Path>,
    changed: bool,
) {
    if !changed || !controller.settings.controls.advance_after_rating {
        return;
    }
    if should_advance_after_rating(controller, primary_row, refocus_path, changed) {
        if controller.random_navigation_mode_enabled() {
            controller.focus_random_visible_sample();
        } else {
            let next_row = primary_row + 1;
            if next_row < controller.ui.browser.viewport.visible.len() {
                controller.focus_browser_row(next_row);
            }
        }
        return;
    }
    if refocus_path
        .and_then(|path| controller.visible_row_for_path(path))
        .is_none()
    {
        let _ = controller.commit_focused_browser_row();
    }
}

fn next_focus_path_for_removed_rows(
    controller: &mut AppController,
    rows: &[usize],
) -> Option<PathBuf> {
    let mut sorted_rows = rows.to_vec();
    sorted_rows.sort_unstable();
    sorted_rows.dedup();
    let first = *sorted_rows.first()?;
    let last = *sorted_rows.last()?;
    let after = last
        .checked_add(1)
        .filter(|idx| *idx < controller.ui.browser.viewport.visible.len())
        .and_then(|idx| controller.ui.browser.viewport.visible.get(idx))
        .and_then(|entry_idx| controller.wav_entry(entry_idx))
        .map(|entry| entry.relative_path.clone());
    if after.is_some() {
        return after;
    }
    first
        .checked_sub(1)
        .and_then(|idx| controller.ui.browser.viewport.visible.get(idx))
        .and_then(|entry_idx| controller.wav_entry(entry_idx))
        .map(|entry| entry.relative_path.clone())
}

pub(crate) fn tag_selected(controller: &mut AppController, target: crate::sample_sources::Rating) {
    let Some(TaggingSelection {
        primary_row,
        refocus_path,
        contexts,
        mut last_error,
    }) = prepare_tagging_selection(controller)
    else {
        return;
    };

    let mut previous_values: Vec<RatingUndoState> = Vec::new();
    let mut applied_updates: Vec<RatingUndoState> = Vec::new();
    for ctx in contexts {
        if ctx.entry.locked {
            continue;
        }
        let target_locked = target == crate::sample_sources::Rating::KEEP_3;
        match controller.set_sample_tag_for_source(
            &ctx.source,
            &ctx.entry.relative_path,
            target,
            true,
        ) {
            Ok(()) => {
                record_applied_update(
                    &ctx,
                    target,
                    target_locked,
                    &mut previous_values,
                    &mut applied_updates,
                );
            }
            Err(err) => last_error = Some(err),
        }
    }

    let tagged_any = !applied_updates.is_empty();
    if tagged_any {
        let label = if target == crate::sample_sources::Rating::KEEP_1 {
            "Tag keep"
        } else if target == crate::sample_sources::Rating::TRASH_3 {
            "Tag trash"
        } else if target == crate::sample_sources::Rating::NEUTRAL {
            "Tag neutral"
        } else {
            "Tag sample"
        };
        push_rating_undo_entry(
            controller,
            label,
            previous_values,
            applied_updates,
            refocus_path.clone(),
        );
    }

    controller.refocus_after_filtered_removal(primary_row);
    if let Some(err) = last_error {
        controller.set_status(err, StatusTone::Error);
    }
    advance_or_commit_after_rating(
        controller,
        primary_row,
        Some(refocus_path.as_path()),
        tagged_any,
    );
}

pub(crate) fn move_selection_column(controller: &mut AppController, delta: isize) {
    use crate::app::state::TriageFlagFilter::*;
    let filters = [All, Keep, Trash, Untagged];
    let current = controller.ui.browser.search.filter;
    let current_idx = filters.iter().position(|f| f == &current).unwrap_or(0) as isize;
    let target_idx = (current_idx + delta).clamp(0, (filters.len() as isize) - 1) as usize;
    let target = filters[target_idx];
    controller.set_browser_filter(target);
}

pub(crate) fn tag_selected_left(controller: &mut AppController) {
    let target = match controller.selected_tag() {
        Some(tag) if tag.is_keep() => crate::sample_sources::Rating::NEUTRAL,
        _ => crate::sample_sources::Rating::TRASH_3,
    };
    controller.tag_selected(target);
}

pub(crate) fn adjust_selected_rating(controller: &mut AppController, delta: i8) {
    let Some(TaggingSelection {
        primary_row,
        refocus_path,
        contexts,
        mut last_error,
    }) = prepare_tagging_selection(controller)
    else {
        return;
    };

    let mut auto_trash_samples: Vec<(SampleSource, WavEntry)> = Vec::new();
    let mut auto_trash_rows = Vec::new();
    let mut previous_values: Vec<RatingUndoState> = Vec::new();
    let mut applied_updates: Vec<RatingUndoState> = Vec::new();

    for ctx in contexts {
        if ctx.entry.locked {
            continue;
        }
        let current_rating = ctx.entry.tag;
        if current_rating == crate::sample_sources::Rating::TRASH_3 && delta < 0 {
            if let Some(row) = controller.visible_row_for_path(&ctx.entry.relative_path) {
                auto_trash_rows.push(row);
            }
            auto_trash_samples.push((ctx.source.clone(), ctx.entry.clone()));
            continue;
        }
        if current_rating == crate::sample_sources::Rating::KEEP_3 && delta > 0 {
            match controller.set_sample_tag_for_source(
                &ctx.source,
                &ctx.entry.relative_path,
                current_rating,
                true,
            ) {
                Ok(()) => {
                    record_applied_update(
                        &ctx,
                        current_rating,
                        true,
                        &mut previous_values,
                        &mut applied_updates,
                    );
                }
                Err(err) => last_error = Some(err),
            }
            continue;
        }

        let mut new_val = current_rating.val() + delta;
        if current_rating.val() != 0 && new_val == 0 {
            new_val += delta;
        }
        let target = crate::sample_sources::Rating::new(new_val.clamp(-3, 3));
        if target != current_rating {
            match controller.set_sample_tag_and_locked_for_source(
                &ctx.source,
                &ctx.entry.relative_path,
                target,
                false,
                true,
            ) {
                Ok(()) => {
                    record_applied_update(
                        &ctx,
                        target,
                        false,
                        &mut previous_values,
                        &mut applied_updates,
                    );
                }
                Err(err) => last_error = Some(err),
            }
        }
    }

    if !applied_updates.is_empty() {
        let label = if delta > 0 {
            "Increase rating"
        } else {
            "Decrease rating"
        };
        push_rating_undo_entry(
            controller,
            label,
            previous_values,
            applied_updates.clone(),
            refocus_path.clone(),
        );
    }

    let auto_trashed = if auto_trash_samples.is_empty() {
        false
    } else {
        let next_focus = next_focus_path_for_removed_rows(controller, &auto_trash_rows);
        controller.move_samples_to_configured_trash(auto_trash_samples, next_focus)
    };

    controller.refocus_after_filtered_removal(primary_row);
    if let Some(err) = last_error {
        controller.set_status(err, StatusTone::Error);
    }
    advance_or_commit_after_rating(
        controller,
        primary_row,
        Some(refocus_path.as_path()),
        !applied_updates.is_empty() || auto_trashed,
    );
}
