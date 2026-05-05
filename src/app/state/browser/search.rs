use std::collections::BTreeSet;

/// Browser playback-age filter chips shown in the native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeFilterChip {
    /// Samples that have never been played.
    NeverPlayed,
    /// Samples whose last playback was at least 30 days ago.
    OlderThanMonth,
    /// Samples whose last playback was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Visual playback-age buckets derived from each sample's `last_played_at` timestamp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeBucket {
    /// Samples played within the last 7 days, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
}

impl PlaybackAgeBucket {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;
    const MONTH_SECS: i64 = 30 * 24 * 60 * 60;

    /// Classify one optional playback timestamp into the browser aging buckets.
    pub fn from_last_played_at(last_played_at: Option<i64>, now_unix_secs: i64) -> Self {
        let Some(last_played_at) = last_played_at else {
            return Self::NeverPlayed;
        };
        if last_played_at >= now_unix_secs {
            return Self::Fresh;
        }
        let age_secs = now_unix_secs.saturating_sub(last_played_at);
        if age_secs >= Self::MONTH_SECS {
            Self::OlderThanMonth
        } else if age_secs >= Self::WEEK_SECS {
            Self::OlderThanWeek
        } else {
            Self::Fresh
        }
    }

    /// Return whether this visual bucket should match one toolbar filter chip.
    pub fn matches_filter_chip(self, chip: PlaybackAgeFilterChip) -> bool {
        match chip {
            PlaybackAgeFilterChip::NeverPlayed => self == Self::NeverPlayed,
            PlaybackAgeFilterChip::OlderThanMonth => self == Self::OlderThanMonth,
            PlaybackAgeFilterChip::OlderThanWeek => self == Self::OlderThanWeek,
        }
    }
}

/// Return the fixed browser playback-age chip order used across UI surfaces.
pub fn browser_playback_age_filter_chips() -> [PlaybackAgeFilterChip; 3] {
    [
        PlaybackAgeFilterChip::NeverPlayed,
        PlaybackAgeFilterChip::OlderThanMonth,
        PlaybackAgeFilterChip::OlderThanWeek,
    ]
}

/// Return whether one playback-age bucket passes the active toolbar chip set.
pub fn playback_age_bucket_matches_filters(
    filters: &BTreeSet<PlaybackAgeFilterChip>,
    bucket: PlaybackAgeBucket,
) -> bool {
    filters.is_empty() || filters.iter().any(|chip| bucket.matches_filter_chip(*chip))
}

/// Return the next Unix-second boundary where one row's playback-age filter
/// match result can change for the active chip set.
pub(crate) fn next_playback_age_filter_change_unix_secs(
    filters: &BTreeSet<PlaybackAgeFilterChip>,
    last_played_at: Option<i64>,
    now_unix_secs: i64,
) -> Option<i64> {
    if filters.is_empty() {
        return None;
    }
    let Some(last_played_at) = last_played_at else {
        return None;
    };

    let current_matches = playback_age_bucket_matches_filters(
        filters,
        PlaybackAgeBucket::from_last_played_at(Some(last_played_at), now_unix_secs),
    );
    for transition_unix_secs in [
        last_played_at.saturating_add(PlaybackAgeBucket::WEEK_SECS),
        last_played_at.saturating_add(PlaybackAgeBucket::MONTH_SECS),
    ] {
        if transition_unix_secs <= now_unix_secs {
            continue;
        }
        let future_matches = playback_age_bucket_matches_filters(
            filters,
            PlaybackAgeBucket::from_last_played_at(Some(last_played_at), transition_unix_secs),
        );
        if future_matches != current_matches {
            return Some(transition_unix_secs);
        }
    }
    None
}

/// Search, filter, and similarity state for the sample browser.
#[derive(Clone, Debug)]
pub struct BrowserSearchState {
    /// Active triage filter.
    pub filter: TriageFlagFilter,
    /// Rating levels selected for filtering (`-3..=3`, plus `4` for locked keeps).
    pub rating_filter: BTreeSet<i8>,
    /// Playback-age chips selected for filtering older or never-played samples.
    pub playback_age_filter: BTreeSet<PlaybackAgeFilterChip>,
    /// Whether only session-marked rows should remain visible.
    pub marked_only: bool,
    /// Optional filter for samples whose filenames are marked as tag-derived.
    pub tag_named_filter: TagNamedFilter,
    /// Text query applied to visible rows via fuzzy search.
    pub search_query: String,
    /// Flag to request focus for the search field in the UI.
    pub search_focus_requested: bool,
    /// When enabled, Up/Down jump through random samples instead of list order.
    pub random_navigation_mode: bool,
    /// Sorting mode for the sample browser list.
    pub sort: SampleBrowserSort,
    /// True when similarity sorting should follow the loaded sample.
    pub similarity_sort_follow_loaded: bool,
    /// Optional similar-sounds filter scoped to the current source.
    pub similar_query: Option<SimilarQuery>,
    /// Near-duplicate highlight set for the focused sample.
    pub focused_similarity: Option<FocusedSimilarity>,
    /// True when a background search/filter job is running.
    pub search_busy: bool,
    /// True when the selected source is still hydrating before browser rows can project.
    pub source_loading: bool,
    /// Latest issued browser search request identifier.
    pub latest_search_request_id: u64,
    /// Latest browser search request identifier applied to visible rows.
    pub latest_applied_search_request_id: u64,
}

impl Default for BrowserSearchState {
    fn default() -> Self {
        Self {
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            playback_age_filter: BTreeSet::new(),
            marked_only: false,
            tag_named_filter: TagNamedFilter::All,
            search_query: String::new(),
            search_focus_requested: false,
            random_navigation_mode: false,
            sort: SampleBrowserSort::ListOrder,
            similarity_sort_follow_loaded: false,
            similar_query: None,
            focused_similarity: None,
            search_busy: false,
            source_loading: false,
            latest_search_request_id: 0,
            latest_applied_search_request_id: 0,
        }
    }
}

/// Holds the current similar-sounds query context.
#[derive(Clone, Debug)]
pub struct SimilarQuery {
    /// Sample id used as the similarity anchor.
    pub sample_id: String,
    /// Display label for the anchor sample.
    pub label: String,
    /// Entry indices in similarity order.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    ///
    /// These are blended similarity values from the resolver pipeline. In
    /// practice they are expected to live near `[-1.0, 1.0]`, but callers may
    /// still pass sentinel values outside that range for unavailable matches.
    pub scores: Vec<f32>,
    /// Optional anchor index in the visible list.
    pub anchor_index: Option<usize>,
}

impl SimilarQuery {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }

    /// Return a normalized similarity strength for UI display.
    ///
    /// The browser bar is intentionally normalized against the current query's
    /// clamped score spread so nearby-but-not-equal results remain visually
    /// distinguishable inside one similarity result set.
    pub fn display_strength_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = self.clamped_score_at(position)?;
        let (min_score, max_score) = self.clamped_score_bounds()?;
        let range = max_score - min_score;
        if range <= f32::EPSILON {
            return Some(Self::absolute_display_strength(score));
        }
        Some(((score - min_score) / range).clamp(0.0, 1.0))
    }

    fn clamped_score_at(&self, position: usize) -> Option<f32> {
        self.scores
            .get(position)
            .copied()
            .map(|score| score.clamp(-1.0, 1.0))
    }

    fn clamped_score_bounds(&self) -> Option<(f32, f32)> {
        let mut scores = self
            .scores
            .iter()
            .copied()
            .map(|score| score.clamp(-1.0, 1.0));
        let first = scores.next()?;
        let mut min_score = first;
        let mut max_score = first;
        for score in scores {
            min_score = min_score.min(score);
            max_score = max_score.max(score);
        }
        Some((min_score, max_score))
    }

    fn absolute_display_strength(score: f32) -> f32 {
        let normalized = ((score.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
        normalized.powf(2.0)
    }
}

/// Highlight metadata for near-duplicate rows relative to the focused sample.
#[derive(Clone, Debug)]
pub struct FocusedSimilarity {
    /// Sample id used as the highlight anchor.
    pub sample_id: String,
    /// Entry indices for near-duplicate matches.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    pub scores: Vec<f32>,
    /// Absolute index of the focused sample, when known.
    pub anchor_index: Option<usize>,
}

impl FocusedSimilarity {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }
}

/// Filter state for tag-derived sample filenames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TagNamedFilter {
    /// Show both tag-derived and unmarked names.
    #[default]
    All,
    /// Show only samples known to be named from tags.
    TagNamed,
    /// Show only samples not yet known to be named from tags.
    NotTagNamed,
}

impl TagNamedFilter {
    /// Return true when one row passes the active tag-name filter.
    pub fn accepts(self, tag_named: bool) -> bool {
        match self {
            Self::All => true,
            Self::TagNamed => tag_named,
            Self::NotTagNamed => !tag_named,
        }
    }

    /// Advance the toolbar chip through off, positive, and negated states.
    pub fn next(self, invert: bool) -> Self {
        match (self, invert) {
            (Self::TagNamed, false) | (Self::NotTagNamed, true) => Self::All,
            (_, false) => Self::TagNamed,
            (_, true) => Self::NotTagNamed,
        }
    }
}

/// Filter options for the single-column sample browser view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagFilter {
    /// Show all triage flags.
    All,
    /// Show keep-only rows.
    Keep,
    /// Show trash-only rows.
    Trash,
    /// Show untagged rows only.
    Untagged,
}

/// Sort modes for the sample browser list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserSort {
    /// Preserve the original list order.
    ListOrder,
    /// Sort by similarity score.
    Similarity,
    /// Sort by playback age ascending.
    PlaybackAgeAsc,
    /// Sort by playback age descending.
    PlaybackAgeDesc,
}

#[cfg(test)]
mod tests {
    use super::SimilarQuery;

    #[test]
    fn similarity_display_strength_uses_query_relative_score_spread() {
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.92, 0.84],
            anchor_index: Some(0),
        };

        let anchor = query
            .display_strength_for_index(0)
            .expect("anchor strength");
        let close = query.display_strength_for_index(1).expect("close strength");
        let far = query.display_strength_for_index(2).expect("far strength");

        assert_eq!(anchor, 1.0);
        assert!(close > far);
        assert!(far < 0.1);
    }

    #[test]
    fn similarity_display_strength_clamps_out_of_range_scores_before_normalizing() {
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.2, -2.0],
            anchor_index: Some(0),
        };

        assert_eq!(query.display_strength_for_index(0), Some(1.0));
        assert_eq!(query.display_strength_for_index(2), Some(0.0));
    }

    #[test]
    fn similarity_display_strength_falls_back_to_absolute_mapping_for_flat_scores() {
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1],
            scores: vec![0.25, 0.25],
            anchor_index: Some(0),
        };

        let expected = ((0.25_f32 + 1.0) * 0.5).powf(2.0);
        assert_eq!(query.display_strength_for_index(0), Some(expected));
        assert_eq!(query.display_strength_for_index(1), Some(expected));
    }
}
