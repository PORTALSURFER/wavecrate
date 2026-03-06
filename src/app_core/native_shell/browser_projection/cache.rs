//! Retained browser-row and selected-path projection cache helpers.

use super::*;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Hash one relative path into a stable row-identity scalar.
pub(in crate::app_core::native_shell) fn browser_row_identity_hash(path: &Path) -> u64 {
    hash_scalar(path)
}

/// Hash one scalar key into a stable 64-bit cache key.
fn hash_scalar<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Clear the retained selected-path lookup cache.
pub(super) fn clear_projected_selected_paths_lookup(controller: &mut AppController) {
    controller.projected_selected_paths_revision =
        Some(controller.ui.browser.selected_paths_revision);
    controller.projected_selected_paths_lookup = None;
}

/// Refresh the retained selected-index bitset when selection changes.
pub(in crate::app_core::native_shell) fn refresh_projected_selected_paths_lookup(
    controller: &mut AppController,
) {
    let selection_revision = controller.ui.browser.selected_paths_revision;
    if controller.ui.browser.selected_paths.is_empty() {
        if controller.projected_selected_paths_lookup.is_some()
            || controller.projected_selected_paths_revision != Some(selection_revision)
        {
            clear_projected_selected_paths_lookup(controller);
        }
        return;
    }
    if controller.projected_selected_paths_revision == Some(selection_revision)
        && controller.projected_selected_paths_lookup.is_some()
    {
        return;
    }
    let lookup = if controller.ui.browser.selected_paths.len() == 1 {
        controller
            .ui
            .browser
            .selected_paths
            .first()
            .cloned()
            .and_then(|path| controller.wav_index_for_path(path.as_path()))
            .map(ProjectedSelectedPathsLookup::Single)
    } else {
        let mut selected_index_lookup = vec![false; controller.wav_entries_len()];
        for selected_path_idx in 0..controller.ui.browser.selected_paths.len() {
            let selected_path = controller.ui.browser.selected_paths[selected_path_idx].clone();
            if let Some(absolute_index) = controller.wav_index_for_path(selected_path.as_path())
                && let Some(selected) = selected_index_lookup.get_mut(absolute_index)
            {
                *selected = true;
            }
        }
        Some(ProjectedSelectedPathsLookup::Dense(selected_index_lookup))
    };
    controller.projected_selected_paths_revision = Some(selection_revision);
    controller.projected_selected_paths_lookup = lookup;
}

/// Return whether one absolute row index is selected in the retained lookup bitset.
pub(in crate::app_core::native_shell) fn selected_index_is_selected(
    controller: &AppController,
    absolute_index: usize,
) -> bool {
    match controller.projected_selected_paths_lookup.as_ref() {
        Some(ProjectedSelectedPathsLookup::Single(selected_index)) => {
            *selected_index == absolute_index
        }
        Some(ProjectedSelectedPathsLookup::Dense(lookup)) => {
            lookup.get(absolute_index).copied().unwrap_or(false)
        }
        None => false,
    }
}

/// Reset retained browser-row projection fields when visible rows changed materially.
pub(in crate::app_core::native_shell) fn refresh_projected_browser_row_cache(
    controller: &mut AppController,
) {
    if controller.projected_browser_rows_revision == controller.ui.browser.visible_rows_revision {
        return;
    }
    controller.projected_browser_rows_revision = controller.ui.browser.visible_rows_revision;
    super::clear_projected_browser_row_cache(controller);
}

/// Return true when one cached browser-row projection still matches the entry snapshot.
fn cached_browser_row_matches_entry(
    cached: &ProjectedBrowserRowCacheEntry,
    row_identity_hash: u64,
    column_index: usize,
    rating_level: i8,
    bucket_label: &str,
    missing: bool,
) -> bool {
    cached.row_identity_hash == row_identity_hash
        && cached.column_index == column_index
        && cached.rating_level == rating_level
        && cached.bucket_label == bucket_label
        && cached.missing == missing
}

/// Resolve static browser-row projection fields from cache, inserting on cache miss.
pub(in crate::app_core::native_shell) fn project_cached_browser_row(
    controller: &mut AppController,
    absolute_index: usize,
) -> Option<(&ProjectedBrowserRowCacheEntry, bool)> {
    let (entry_tag, row_identity_hash, missing, looped, relative_path) =
        controller.wav_entry(absolute_index).map(|entry| {
            (
                entry.tag,
                browser_row_identity_hash(entry.relative_path.as_path()),
                entry.missing,
                entry.looped,
                entry.relative_path.clone(),
            )
        })?;
    let column_index = super::browser_column_index(entry_tag);
    let rating_level = entry_tag.val();
    let bucket_label =
        super::browser_bucket_label(controller, absolute_index, relative_path.as_path(), looped);
    let cache_hit = controller
        .projected_browser_rows
        .get(&absolute_index)
        .is_some_and(|cached| {
            cached_browser_row_matches_entry(
                cached,
                row_identity_hash,
                column_index,
                rating_level,
                &bucket_label,
                missing,
            )
        });
    trace_browser_row_cache_lookup(cache_hit);
    if !cache_hit {
        let row_label = controller
            .label_for_ref(absolute_index)
            .map(str::to_string)
            .unwrap_or_else(|| view_model::sample_display_label(relative_path.as_path()));
        let cached = ProjectedBrowserRowCacheEntry {
            row_identity_hash,
            row_label,
            column_index,
            rating_level,
            bucket_label,
            missing,
        };
        if controller.projected_browser_rows.len() >= MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE {
            super::clear_projected_browser_row_cache(controller);
        }
        controller
            .projected_browser_rows
            .insert(absolute_index, cached);
    }
    let projected = controller.projected_browser_rows.get(&absolute_index)?;
    Some((
        projected,
        selected_index_is_selected(controller, absolute_index),
    ))
}
