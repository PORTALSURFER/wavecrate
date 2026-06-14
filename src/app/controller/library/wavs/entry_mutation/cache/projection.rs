use super::*;
use std::path::Path;

use super::source_entries::{EntryCacheMutation, EntryInsertion};

pub(super) fn apply_same_path_metadata_projection(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    new_entry: &WavEntry,
    mutation: EntryCacheMutation,
) {
    if !mutation.updated
        || controller.selection_state.ctx.selected_source.as_ref() != Some(&source.id)
    {
        return;
    }
    let source_revision = source_revision(controller, source);
    let patched = mutation
        .selected_index
        .and_then(|index| {
            controller
                .ui_cache
                .browser
                .pipeline
                .update_entry_metadata(index, new_entry)
                .then_some(index)
        })
        .is_some();
    controller
        .ui_cache
        .browser
        .pipeline
        .sync_source_revision(source_revision);
    if !patched {
        controller.ui_cache.browser.pipeline.invalidate();
    }
    controller.rebuild_browser_lists_with_metadata_delta(vec![path.to_path_buf()]);
}

pub(super) fn apply_path_change_projection(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_entry: &WavEntry,
    keep_visible_order: bool,
    mutation: EntryCacheMutation,
) {
    if !mutation.updated {
        controller.invalidate_wav_entries_for_source_preserve_folders(source);
        return;
    }
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
        let source_revision = source_revision(controller, source);
        let patched_pipeline = mutation
            .selected_index
            .and_then(|index| {
                controller
                    .ui_cache
                    .browser
                    .pipeline
                    .update_entry_snapshot(index, new_entry)
                    .then_some(index)
            })
            .is_some();
        controller
            .ui_cache
            .browser
            .pipeline
            .sync_source_revision(source_revision);
        if !patched_pipeline {
            controller.ui_cache.browser.pipeline.invalidate();
        }
        if keep_visible_order {
            preserve_browser_order_for_path_change(controller, source, new_entry, mutation);
        } else {
            rebuild_browser_for_path_change(controller);
        }
    }
    if old_path != new_entry.relative_path
        && let Some(index) = mutation.selected_index
    {
        controller.update_cached_browser_label_for_index(
            &source.id,
            index,
            &new_entry.relative_path,
        );
    }
}

pub(super) fn apply_insert_projection(
    controller: &mut AppController,
    source: &SampleSource,
    insertion: EntryInsertion,
) {
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
        if insertion.selected_entries_loaded && insertion.selected_inserted {
            if let Some(index) = insertion.selected_insert_index {
                controller.insert_cached_browser_label_slot(&source.id, index);
            } else {
                controller.ui_cache.browser.labels.remove(&source.id);
            }
            invalidate_browser_insert_metadata_caches(controller, &source.id);
            controller.rebuild_browser_lists();
        } else if insertion.selected_entries_loaded {
            controller.invalidate_wav_entries_for_source(source);
        } else {
            invalidate_browser_insert_all_caches(controller, &source.id);
        }
    } else {
        controller.ui_cache.browser.labels.remove(&source.id);
        controller.ui_cache.browser.bpm_values.remove(&source.id);
    }
}

fn source_revision(controller: &mut AppController, source: &SampleSource) -> Option<u64> {
    controller
        .database_for(source)
        .ok()
        .and_then(|db| db.get_revision().ok())
}

fn preserve_browser_order_for_path_change(
    controller: &mut AppController,
    source: &SampleSource,
    new_entry: &WavEntry,
    mutation: EntryCacheMutation,
) {
    if let Some(index) = mutation.selected_index {
        controller.update_cached_browser_label_for_index(
            &source.id,
            index,
            &new_entry.relative_path,
        );
        controller.projected_browser_rows.remove(&index);
    }
    controller.refresh_browser_selection_markers();
    controller.mark_browser_row_metadata_projection_revision_dirty();
}

fn rebuild_browser_for_path_change(controller: &mut AppController) {
    controller.ui_cache.browser.search.invalidate();
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_search_projection_revision_dirty();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.rebuild_browser_lists();
}

fn invalidate_browser_insert_metadata_caches(controller: &mut AppController, source_id: &SourceId) {
    controller.ui_cache.browser.bpm_values.remove(source_id);
    controller.ui_cache.browser.search.invalidate();
    controller.ui_cache.browser.pipeline.invalidate();
}

fn invalidate_browser_insert_all_caches(controller: &mut AppController, source_id: &SourceId) {
    controller.ui_cache.browser.labels.remove(source_id);
    invalidate_browser_insert_metadata_caches(controller, source_id);
}
