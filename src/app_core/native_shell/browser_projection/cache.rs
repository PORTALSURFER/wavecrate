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
        Some(controller.ui.browser.selection.selected_paths_revision);
    controller.projected_selected_paths_lookup = None;
}

/// Refresh the retained selected-index bitset when selection changes.
pub(in crate::app_core::native_shell) fn refresh_projected_selected_paths_lookup(
    controller: &mut AppController,
) {
    let selection_revision = controller.ui.browser.selection.selected_paths_revision;
    let selected_indices = controller.browser_selected_indices_snapshot();
    if selected_indices.is_empty() {
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
    let lookup = if selected_indices.len() == 1 {
        selected_indices
            .first()
            .copied()
            .map(ProjectedSelectedPathsLookup::Single)
    } else {
        let mut selected_index_lookup = vec![false; controller.wav_entries_len()];
        for &absolute_index in &selected_indices {
            if let Some(selected) = selected_index_lookup.get_mut(absolute_index) {
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
    let source_id = controller.selected_source_id();
    if controller.projected_browser_rows_source_id == source_id {
        return;
    }
    controller.projected_browser_rows_source_id = source_id;
    super::clear_projected_browser_row_cache(controller);
}

/// Stable snapshot of one row's derived cache fields.
struct BrowserRowCacheFingerprint {
    row_identity_hash: u64,
    column_index: usize,
    rating_level: i8,
    playback_age_bucket: PlaybackAgeBucket,
    missing: bool,
    looped: bool,
    locked: bool,
    marked: bool,
    bpm_value_bits: Option<u32>,
    long_sample_mark: bool,
}

/// Return true when one cached browser-row projection still matches the entry snapshot.
fn cached_browser_row_matches_entry(
    cached: &ProjectedBrowserRowCacheEntry,
    fingerprint: &BrowserRowCacheFingerprint,
) -> bool {
    cached.row_identity_hash == fingerprint.row_identity_hash
        && cached.column_index == fingerprint.column_index
        && cached.rating_level == fingerprint.rating_level
        && cached.playback_age_bucket == fingerprint.playback_age_bucket
        && cached.missing == fingerprint.missing
        && cached.looped == fingerprint.looped
        && cached.locked == fingerprint.locked
        && cached.marked == fingerprint.marked
        && cached.bpm_value_bits == fingerprint.bpm_value_bits
        && cached.long_sample_mark == fingerprint.long_sample_mark
}

/// Return the next retained browser-row cache usage tick.
fn next_projected_browser_row_cache_tick(controller: &mut AppController) -> u64 {
    let next = controller
        .projected_browser_row_cache_clock
        .wrapping_add(1)
        .max(1);
    controller.projected_browser_row_cache_clock = next;
    next
}

/// Evict one least-recently-used retained browser row instead of clearing the full cache.
fn evict_least_recently_used_browser_row(controller: &mut AppController) {
    let Some((&absolute_index, _)) = controller
        .projected_browser_rows
        .iter()
        .min_by_key(|(_, cached)| cached.last_used_tick)
    else {
        return;
    };
    controller.projected_browser_rows.remove(&absolute_index);
}

/// Resolve static browser-row projection fields from cache, inserting on cache miss.
pub(in crate::app_core::native_shell) fn project_cached_browser_row(
    controller: &mut AppController,
    absolute_index: usize,
    playback_age_now_unix_secs: i64,
) -> Option<(&ProjectedBrowserRowCacheEntry, bool)> {
    let selected_source_id = controller.selected_source_id();
    let entry = controller.wav_entry(absolute_index)?;
    let relative_path = entry.relative_path.clone();
    let entry_tag = entry.tag;
    let row_identity_hash = browser_row_identity_hash(relative_path.as_path());
    let missing = entry.missing;
    let looped = entry.looped;
    let locked = entry.locked;
    let last_played_at = entry.last_played_at;
    let marked = selected_source_id
        .as_ref()
        .is_some_and(|source_id| controller.browser_sample_marked(source_id, &relative_path));
    let column_index = super::browser_column_index(entry_tag);
    let rating_level = entry_tag.val();
    let playback_age_bucket =
        PlaybackAgeBucket::from_last_played_at(last_played_at, playback_age_now_unix_secs);
    let long_sample_mark = controller
        .cached_feature_status_for_entry(absolute_index)
        .and_then(|status| status.long_sample_mark)
        == Some(true);
    let bpm_value = controller.bpm_value_for_path(relative_path.as_path());
    let bpm_value_bits = bpm_value.map(f32::to_bits);
    let fingerprint = BrowserRowCacheFingerprint {
        row_identity_hash,
        column_index,
        rating_level,
        playback_age_bucket,
        missing,
        looped,
        locked,
        marked,
        bpm_value_bits,
        long_sample_mark,
    };
    let cache_hit = controller
        .projected_browser_rows
        .get(&absolute_index)
        .is_some_and(|cached| cached_browser_row_matches_entry(cached, &fingerprint));
    trace_browser_row_cache_lookup(cache_hit);
    let row_label = (!cache_hit).then(|| {
        controller
            .label_for_ref(absolute_index)
            .map(str::to_string)
            .unwrap_or_else(|| view_model::sample_display_label(relative_path.as_path()))
    });
    let relative_path_for_insert = (!cache_hit).then_some(relative_path);
    let last_used_tick = next_projected_browser_row_cache_tick(controller);
    if !cache_hit {
        let bucket_label = super::browser_bucket_label(bpm_value, looped, long_sample_mark);
        let cached = ProjectedBrowserRowCacheEntry {
            row_identity_hash,
            relative_path: relative_path_for_insert
                .expect("cache miss should retain one relative path"),
            row_label: row_label.expect("cache miss should retain one row label"),
            column_index,
            rating_level,
            playback_age_bucket,
            bucket_label,
            missing,
            looped,
            locked,
            marked,
            bpm_value_bits,
            long_sample_mark,
            last_used_tick,
        };
        if controller.projected_browser_rows.len() >= MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE {
            evict_least_recently_used_browser_row(controller);
        }
        controller
            .projected_browser_rows
            .insert(absolute_index, cached);
    } else if let Some(cached) = controller.projected_browser_rows.get_mut(&absolute_index) {
        cached.last_used_tick = last_used_tick;
    }
    let projected = controller.projected_browser_rows.get(&absolute_index)?;
    Some((
        projected,
        selected_index_is_selected(controller, absolute_index),
    ))
}
