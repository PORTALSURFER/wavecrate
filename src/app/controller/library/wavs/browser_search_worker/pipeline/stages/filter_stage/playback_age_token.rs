//! Time-bound invalidation tokens for playback-age filter cache identity.

use super::super::super::*;
use super::cache_key::hash_value;

pub(super) fn playback_age_filter_cache_token(
    cache: &mut SearchWorkerCache,
    filters: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    now_unix_secs: i64,
) -> Option<i64> {
    if filters.is_empty() {
        return None;
    }
    let filter_hash = hash_value(filters);
    if let Some(cached) = cache
        .playback_age_token_caches
        .iter()
        .copied()
        .find(|cached| cached.revision == cache.revision && cached.filter_hash == filter_hash)
        && cached.token.is_none_or(|token| now_unix_secs < token)
    {
        return cached.token;
    }

    let token = cache
        .entries
        .as_ref()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            crate::app::state::next_playback_age_filter_change_unix_secs(
                filters,
                entry.last_played_at,
                now_unix_secs,
            )
        })
        .min();
    let cache_entry = WorkerPlaybackAgeTokenCache {
        revision: cache.revision,
        filter_hash,
        token,
    };
    if let Some(index) = cache
        .playback_age_token_caches
        .iter()
        .position(|cached| cached.revision == cache.revision && cached.filter_hash == filter_hash)
    {
        cache.playback_age_token_caches[index] = cache_entry;
    } else {
        cache.playback_age_token_caches.push(cache_entry);
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn playback_age_filter_token_reuses_cached_boundary() {
        let mut cache = SearchWorkerCache {
            revision: 7,
            entries: Some(vec![CompactSearchEntry {
                display_label: "aging".into(),
                relative_path: "aging.wav".into(),
                tag: Rating::NEUTRAL,
                locked: false,
                last_played_at: Some(100),
                tag_named: false,
            }]),
            ..SearchWorkerCache::default()
        };
        let filters = BTreeSet::from([crate::app::state::PlaybackAgeFilterChip::OlderThanWeek]);
        let before =
            playback_age_filter_cache_token(&mut cache, &filters, 100 + (7 * 24 * 60 * 60) - 2);
        let again =
            playback_age_filter_cache_token(&mut cache, &filters, 100 + (7 * 24 * 60 * 60) - 1);

        assert_eq!(before, Some(100 + (7 * 24 * 60 * 60)));
        assert_eq!(again, before);
        assert_eq!(cache.playback_age_token_caches.len(), 1);
    }
}
