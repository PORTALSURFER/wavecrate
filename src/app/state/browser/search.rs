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
    /// Similarity scores aligned with `indices` (0.0 = least similar, 1.0 = most similar).
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
    pub fn display_strength_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = *self.scores.get(position)?;
        let normalized = ((score.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
        Some(normalized.powf(2.0))
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
