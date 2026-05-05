use std::sync::Arc;

/// Cached fuzzy-score payload for one scope/query pair.
#[derive(Clone)]
pub(crate) struct QueryScoreCacheEntry<Scope> {
    /// Search scope associated with the score vector.
    pub(crate) scope: Scope,
    /// Exact query string associated with the score vector.
    pub(crate) query: String,
    /// Score vector aligned to absolute entry indices.
    pub(crate) scores: Arc<[Option<i64>]>,
    /// Absolute entry indices whose labels matched `query`.
    pub(crate) matched_indices: Arc<[usize]>,
}

/// One candidate-step outcome while computing fuzzy scores.
pub(crate) enum ScoreCandidateResult {
    /// Continue scoring, optionally storing a fuzzy-match score for the candidate.
    Continue(Option<i64>),
    /// Stop scoring because the caller invalidated the current pass.
    Cancel,
}

/// Promote an exact cache hit to the front of the bounded LRU list.
pub(crate) fn promote_exact_query_score_cache_entry<Scope>(
    cache: &mut Vec<QueryScoreCacheEntry<Scope>>,
    scope: &Scope,
    query: &str,
    entries_len: usize,
) -> Option<QueryScoreCacheEntry<Scope>>
where
    Scope: Clone + PartialEq,
{
    let cached_index = cache.iter().position(|entry| {
        entry.scope == *scope && entry.query == query && entry.scores.len() == entries_len
    })?;
    let cached = cache.remove(cached_index);
    cache.insert(0, cached.clone());
    Some(cached)
}

/// Return the longest reusable prefix-query cache entry for `query`.
pub(crate) fn reusable_prefix_query_score_cache_entry<Scope>(
    cache: &[QueryScoreCacheEntry<Scope>],
    scope: &Scope,
    query: &str,
    entries_len: usize,
) -> Option<QueryScoreCacheEntry<Scope>>
where
    Scope: Clone + PartialEq,
{
    cache
        .iter()
        .filter(|entry| {
            entry.scope == *scope
                && entry.scores.len() == entries_len
                && !entry.query.is_empty()
                && query.starts_with(&entry.query)
                && query.len() > entry.query.len()
        })
        .max_by_key(|entry| entry.query.len())
        .cloned()
}

/// Score either all entries or a reusable matched-index subset into `scores`.
///
/// Returns `None` when the caller cancels the pass.
pub(crate) fn score_query_candidates(
    scores: &mut [Option<i64>],
    candidate_indices: Option<&[usize]>,
    entries_len: usize,
    mut score_candidate: impl FnMut(usize, usize) -> ScoreCandidateResult,
) -> Option<Arc<[usize]>> {
    let mut matched_indices = Vec::new();
    if let Some(candidate_indices) = candidate_indices {
        for (offset, &index) in candidate_indices.iter().enumerate() {
            if !apply_candidate_score(
                scores,
                index,
                offset,
                &mut matched_indices,
                &mut score_candidate,
            ) {
                return None;
            }
        }
    } else {
        for index in 0..entries_len {
            if !apply_candidate_score(
                scores,
                index,
                index,
                &mut matched_indices,
                &mut score_candidate,
            ) {
                return None;
            }
        }
    }
    Some(matched_indices.into())
}

/// Insert a freshly computed cache entry at the head of the bounded LRU list.
pub(crate) fn store_query_score_cache_entry<Scope>(
    cache: &mut Vec<QueryScoreCacheEntry<Scope>>,
    max_cached_queries: usize,
    scope: Scope,
    query: String,
    scores: Arc<[Option<i64>]>,
    matched_indices: Arc<[usize]>,
) {
    cache.insert(
        0,
        QueryScoreCacheEntry {
            scope,
            query,
            scores,
            matched_indices,
        },
    );
    cache.truncate(max_cached_queries);
}

fn apply_candidate_score(
    scores: &mut [Option<i64>],
    index: usize,
    offset: usize,
    matched_indices: &mut Vec<usize>,
    score_candidate: &mut impl FnMut(usize, usize) -> ScoreCandidateResult,
) -> bool {
    match score_candidate(index, offset) {
        ScoreCandidateResult::Continue(score) => {
            if index < scores.len() {
                scores[index] = score;
                if score.is_some() {
                    matched_indices.push(index);
                }
            }
            true
        }
        ScoreCandidateResult::Cancel => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promote_exact_query_score_cache_entry_moves_hit_to_front() {
        let mut cache = vec![
            QueryScoreCacheEntry {
                scope: "source-a",
                query: String::from("snare"),
                scores: Arc::from([Some(10), Some(8)]),
                matched_indices: Arc::from([0, 1]),
            },
            QueryScoreCacheEntry {
                scope: "source-a",
                query: String::from("kick"),
                scores: Arc::from([Some(4), Some(3)]),
                matched_indices: Arc::from([0, 1]),
            },
        ];

        let reused = promote_exact_query_score_cache_entry(&mut cache, &"source-a", "kick", 2)
            .expect("expected exact query cache hit");

        assert_eq!(reused.scores.as_ref(), [Some(4), Some(3)]);
        assert_eq!(cache[0].query, "kick");
    }

    #[test]
    fn reusable_prefix_query_score_cache_entry_prefers_longest_match() {
        let cache = vec![
            QueryScoreCacheEntry {
                scope: "source-a",
                query: String::from("k"),
                scores: Arc::from([Some(10), None]),
                matched_indices: Arc::from([0]),
            },
            QueryScoreCacheEntry {
                scope: "source-a",
                query: String::from("ki"),
                scores: Arc::from([Some(9), None]),
                matched_indices: Arc::from([0]),
            },
        ];

        let reused = reusable_prefix_query_score_cache_entry(&cache, &"source-a", "kick", 2)
            .expect("expected reusable prefix");

        assert_eq!(reused.query, "ki");
        assert_eq!(reused.matched_indices.as_ref(), &[0]);
    }

    #[test]
    fn score_query_candidates_reuses_candidate_subset() {
        let mut scores = vec![None; 4];
        let matched = score_query_candidates(&mut scores, Some(&[1, 3]), 4, |index, _| {
            ScoreCandidateResult::Continue(match index {
                1 => Some(30),
                3 => Some(10),
                _ => None,
            })
        })
        .expect("expected scoring to finish");

        assert_eq!(scores, vec![None, Some(30), None, Some(10)]);
        assert_eq!(matched.as_ref(), &[1, 3]);
    }

    #[test]
    fn score_query_candidates_stops_when_canceled() {
        let mut scores = vec![None; 3];
        let matched = score_query_candidates(&mut scores, None, 3, |index, _| {
            if index == 1 {
                ScoreCandidateResult::Cancel
            } else {
                ScoreCandidateResult::Continue(Some(index as i64))
            }
        });

        assert!(matched.is_none());
        assert_eq!(scores[0], Some(0));
        assert_eq!(scores[1], None);
        assert_eq!(scores[2], None);
    }
}
