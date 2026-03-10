use super::*;
use crate::app::state::{SampleBrowserSort, TriageFlagFilter};
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
    tag: crate::sample_sources::Rating,
    locked: bool,
) -> bool {
    let triage_ok = match filter {
        TriageFlagFilter::All => true,
        TriageFlagFilter::Keep => tag.is_keep(),
        TriageFlagFilter::Trash => tag.is_trash(),
        TriageFlagFilter::Untagged => tag.is_neutral(),
    };
    let rating_ok = rating_filter.is_empty()
        || rating_filter.contains(&tag.val())
        || (locked && rating_filter.contains(&4));
    triage_ok && rating_ok
}

/// Sort visible row indices by playback age then by absolute index.
pub(super) fn sort_visible_by_playback_age(
    controller: &mut AppController,
    visible: &mut [usize],
    ascending: bool,
) {
    visible.sort_by(|a, b| {
        let a_key = controller
            .wav_entry(*a)
            .and_then(|entry| entry.last_played_at)
            .unwrap_or(i64::MIN);
        let b_key = controller
            .wav_entry(*b)
            .and_then(|entry| entry.last_played_at)
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

/// Convert root-folder mode into a stable scalar for cache keys.
pub(super) fn root_mode_key(mode: crate::app::state::RootFolderFilterMode) -> u8 {
    match mode {
        crate::app::state::RootFolderFilterMode::AllDescendants => 0,
        crate::app::state::RootFolderFilterMode::RootOnly => 1,
    }
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
