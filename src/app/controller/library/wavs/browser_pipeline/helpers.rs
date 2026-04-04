use super::*;
use crate::app::state::{
    PlaybackAgeBucket, PlaybackAgeFilterChip, SampleBrowserSort, TriageFlagFilter,
    playback_age_bucket_matches_filters,
};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Apply explicit sort policy for similarity-query result rows.
pub(super) fn apply_sort_for_similar(
    controller: &mut AppController,
    visible: &mut [usize],
    sort_mode: SampleBrowserSort,
    similar: &crate::app::state::SimilarQuery,
) {
    match sort_mode {
        SampleBrowserSort::Similarity => {
            let mut lookup = vec![None; controller.wav_entries_len()];
            for (&index, &score) in similar.indices.iter().zip(similar.scores.iter()) {
                if index < lookup.len() {
                    lookup[index] = Some(score);
                }
            }
            visible.sort_by(|a, b| {
                let a_score = lookup
                    .get(*a)
                    .and_then(|score| *score)
                    .unwrap_or(f32::NEG_INFINITY);
                let b_score = lookup
                    .get(*b)
                    .and_then(|score| *score)
                    .unwrap_or(f32::NEG_INFINITY);
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.cmp(b))
            });
            if let Some(anchor) = similar.anchor_index
                && let Some(pos) = visible.iter().position(|index| *index == anchor)
            {
                visible.rotate_right(visible.len().saturating_sub(pos));
            }
        }
        SampleBrowserSort::PlaybackAgeAsc => {
            sort_visible_by_playback_age(controller, visible, true);
        }
        SampleBrowserSort::PlaybackAgeDesc => {
            sort_visible_by_playback_age(controller, visible, false);
        }
        SampleBrowserSort::ListOrder => {
            visible.sort_unstable();
        }
    }
}

/// Return whether an entry tag matches the active triage/rating filters.
pub(super) fn filter_accepts(
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    playback_age_filter: &std::collections::BTreeSet<PlaybackAgeFilterChip>,
    marked_only: bool,
    marked: bool,
    tag: crate::sample_sources::Rating,
    locked: bool,
    last_played_at: Option<i64>,
    playback_age_now_unix_secs: i64,
) -> bool {
    let triage_ok = match filter {
        TriageFlagFilter::All => true,
        TriageFlagFilter::Keep => tag.is_keep(),
        TriageFlagFilter::Trash => tag.is_trash(),
        TriageFlagFilter::Untagged => tag.is_neutral(),
    };
    let rating_level = browser_rating_filter_level(tag, locked);
    let rating_ok = rating_filter.is_empty() || rating_filter.contains(&rating_level);
    let playback_age_bucket =
        PlaybackAgeBucket::from_last_played_at(last_played_at, playback_age_now_unix_secs);
    let playback_age_ok =
        playback_age_bucket_matches_filters(playback_age_filter, playback_age_bucket);
    let marked_ok = !marked_only || marked;
    triage_ok && rating_ok && playback_age_ok && marked_ok
}

/// Return the effective browser rating-filter level for one sample row.
///
/// Locked keeps occupy their own filter chip (`4`) and should not also match the
/// ordinary `KEEP_3` filter level.
fn browser_rating_filter_level(tag: crate::sample_sources::Rating, locked: bool) -> i8 {
    if locked && tag.is_keep() {
        4
    } else {
        tag.val()
    }
}

/// Sort visible row indices by playback age then by absolute index.
pub(super) fn sort_visible_by_playback_age(
    controller: &mut AppController,
    visible: &mut [usize],
    ascending: bool,
) {
    let compact_entries = &controller.ui_cache.browser.pipeline.compact_entries;
    visible.sort_by(|a, b| {
        let a_key = compact_entries
            .get(*a)
            .and_then(|entry| entry.last_played_at)
            .or_else(|| {
                controller
                    .wav_entries
                    .entry(*a)
                    .and_then(|entry| entry.last_played_at)
            })
            .unwrap_or(i64::MIN);
        let b_key = compact_entries
            .get(*b)
            .and_then(|entry| entry.last_played_at)
            .or_else(|| {
                controller
                    .wav_entries
                    .entry(*b)
                    .and_then(|entry| entry.last_played_at)
            })
            .unwrap_or(i64::MIN);
        let order = if ascending {
            a_key.cmp(&b_key)
        } else {
            b_key.cmp(&a_key)
        };
        order.then_with(|| a.cmp(b))
    });
}

/// Hash any value into a compact stage fingerprint.
pub(super) fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Convert triage-filter enum into a stable scalar for cache keys.
pub(super) fn filter_key(filter: TriageFlagFilter) -> u8 {
    match filter {
        TriageFlagFilter::All => 0,
        TriageFlagFilter::Keep => 1,
        TriageFlagFilter::Trash => 2,
        TriageFlagFilter::Untagged => 3,
    }
}

/// Convert browser-sort enum into a stable scalar for cache keys.
pub(super) fn sort_key(sort: SampleBrowserSort) -> u8 {
    match sort {
        SampleBrowserSort::ListOrder => 0,
        SampleBrowserSort::Similarity => 1,
        SampleBrowserSort::PlaybackAgeAsc => 2,
        SampleBrowserSort::PlaybackAgeDesc => 3,
    }
}

/// Hash a similarity query payload for stage cache invalidation.
pub(super) fn similarity_fingerprint(query: &crate::app::state::SimilarQuery) -> u64 {
    hash_value(&(
        &query.sample_id,
        &query.label,
        &query.indices,
        query
            .scores
            .iter()
            .map(|score| score.to_bits())
            .collect::<Vec<u32>>(),
        query.anchor_index,
    ))
}

/// Return the next playback-age timestamp that should invalidate the cached
/// filtered-stage rows for the current base snapshot.
pub(super) fn playback_age_filter_cache_token(
    controller: &mut AppController,
    filters: &std::collections::BTreeSet<PlaybackAgeFilterChip>,
    now_unix_secs: i64,
) -> Option<i64> {
    if filters.is_empty() {
        return None;
    }
    let base_fingerprint_hash = hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let filter_hash = hash_value(filters);
    if let Some(cached) = controller
        .ui_cache
        .browser
        .pipeline
        .playback_age_token_cache
        && cached.base_fingerprint_hash == base_fingerprint_hash
        && cached.filter_hash == filter_hash
        && cached.token.is_none_or(|token| now_unix_secs < token)
    {
        return cached.token;
    }

    let token = controller
        .ui_cache
        .browser
        .pipeline
        .base_rows
        .iter()
        .filter_map(|index| {
            controller
                .ui_cache
                .browser
                .pipeline
                .compact_entries
                .get(*index)
                .and_then(|entry| {
                    crate::app::state::next_playback_age_filter_change_unix_secs(
                        filters,
                        entry.last_played_at,
                        now_unix_secs,
                    )
                })
        })
        .min();
    controller
        .ui_cache
        .browser
        .pipeline
        .playback_age_token_cache = Some(super::PlaybackAgeTokenCache {
        base_fingerprint_hash,
        filter_hash,
        token,
    });
    token
}
