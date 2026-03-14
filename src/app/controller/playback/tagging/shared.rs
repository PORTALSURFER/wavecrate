use super::*;

#[derive(Clone)]
pub(super) struct RatingUndoState {
    source_id: SourceId,
    path: PathBuf,
    tag: crate::sample_sources::Rating,
    locked: bool,
}

pub(super) struct TaggingSelection {
    pub(super) primary_row: usize,
    pub(super) refocus_path: PathBuf,
    pub(super) contexts:
        Vec<crate::app::controller::library::browser_controller::helpers::TriageSampleContext>,
    pub(super) last_error: Option<String>,
}

pub(super) fn prepare_tagging_selection(
    controller: &mut AppController,
) -> Option<TaggingSelection> {
    let selected_index = controller.selected_row_index()?;
    let refocus_path = controller
        .wav_entry(selected_index)
        .map(|entry| entry.relative_path.clone())?;
    let primary_row = controller.visible_row_for_path(&refocus_path)?;
    let rows = controller.action_rows_from_primary(primary_row);
    controller.focus_browser_context();
    controller.ui.browser.autoscroll = true;
    let (contexts, last_error) = collect_unique_contexts(controller, rows);
    Some(TaggingSelection {
        primary_row,
        refocus_path,
        contexts,
        last_error,
    })
}

fn collect_unique_contexts(
    controller: &mut AppController,
    rows: Vec<usize>,
) -> (
    Vec<crate::app::controller::library::browser_controller::helpers::TriageSampleContext>,
    Option<String>,
) {
    let mut contexts = Vec::with_capacity(rows.len());
    let mut seen = std::collections::HashSet::new();
    let mut last_error = None;
    for row in rows {
        match controller.resolve_browser_sample(row) {
            Ok(ctx) => {
                if seen.insert(ctx.entry.relative_path.clone()) {
                    contexts.push(ctx);
                }
            }
            Err(err) => last_error = Some(err),
        }
    }
    (contexts, last_error)
}

pub(super) fn record_applied_update(
    ctx: &crate::app::controller::library::browser_controller::helpers::TriageSampleContext,
    tag: crate::sample_sources::Rating,
    locked: bool,
    previous_values: &mut Vec<RatingUndoState>,
    applied_updates: &mut Vec<RatingUndoState>,
) {
    previous_values.push(RatingUndoState {
        source_id: ctx.source.id.clone(),
        path: ctx.entry.relative_path.clone(),
        tag: ctx.entry.tag,
        locked: ctx.entry.locked,
    });
    applied_updates.push(RatingUndoState {
        source_id: ctx.source.id.clone(),
        path: ctx.entry.relative_path.clone(),
        tag,
        locked,
    });
}

pub(super) fn push_rating_undo_entry(
    controller: &mut AppController,
    label: &'static str,
    previous_values: Vec<RatingUndoState>,
    applied_updates: Vec<RatingUndoState>,
    refocus_path: PathBuf,
) {
    let redo_updates = applied_updates.clone();
    controller.push_undo_entry(super::undo::UndoEntry::<AppController>::new(
        label,
        move |controller: &mut AppController| {
            apply_rating_updates(controller, &previous_values)?;
            refocus_visible_path(controller, &refocus_path);
            Ok(super::undo::UndoExecution::Applied)
        },
        move |controller: &mut AppController| {
            apply_rating_updates(controller, &redo_updates)?;
            Ok(super::undo::UndoExecution::Applied)
        },
    ));
}

fn apply_rating_updates(
    controller: &mut AppController,
    updates: &[RatingUndoState],
) -> Result<(), String> {
    for update in updates {
        let source = controller
            .library
            .sources
            .iter()
            .find(|s| s.id == update.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        controller.set_sample_tag_and_locked_for_source(
            &source,
            &update.path,
            update.tag,
            update.locked,
            false,
        )?;
    }
    Ok(())
}

fn refocus_visible_path(controller: &mut AppController, path: &Path) {
    if let Some(row) = controller.visible_row_for_path(path) {
        controller.focus_browser_row_only(row);
    }
}
