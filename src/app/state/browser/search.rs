use std::collections::BTreeSet;

mod playback_age;
mod sidebar;
mod similarity;

pub(crate) use playback_age::next_playback_age_filter_change_unix_secs;
pub use playback_age::{
    PlaybackAgeBucket, PlaybackAgeFilterChip, browser_playback_age_filter_chips,
    playback_age_bucket_matches_filters,
};
#[allow(unused_imports)]
pub use sidebar::{
    BrowserBitDepthFacet, BrowserBpmFacet, BrowserChannelFacet, BrowserFormatFacet,
    BrowserKeyFacet, BrowserSidebarFilterFacet, BrowserSidebarFilterOption,
    BrowserSidebarFilterState,
};
#[allow(unused_imports)]
pub use similarity::{
    EMPTY_SIMILARITY_ASPECT_SCORE_ROW, FocusedSimilarity, SimilarQuery, SimilarityAspectScoreRow,
    empty_similarity_aspect_score_rows,
};

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
    /// Sidebar metadata facets applied alongside rating and playback-age filters.
    pub sidebar_filters: BrowserSidebarFilterState,
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
            sidebar_filters: BrowserSidebarFilterState::default(),
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
    use super::{
        BrowserBpmFacet, BrowserFormatFacet, BrowserSidebarFilterOption, BrowserSidebarFilterState,
        SimilarQuery, empty_similarity_aspect_score_rows,
    };
    use crate::sample_sources::config::SimilarityAspectSettings;
    use std::path::Path;
    use wavecrate_analysis::aspects::SimilarityAspect;

    #[test]
    fn similarity_display_strength_uses_query_relative_score_spread() {
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.92, 0.84],
            aspect_scores: empty_similarity_aspect_score_rows(3),
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
            aspect_scores: empty_similarity_aspect_score_rows(3),
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
            aspect_scores: empty_similarity_aspect_score_rows(2),
            anchor_index: Some(0),
        };

        let expected = ((0.25_f32 + 1.0) * 0.5).powf(2.0);
        assert_eq!(query.display_strength_for_index(0), Some(expected));
        assert_eq!(query.display_strength_for_index(1), Some(expected));
    }

    #[test]
    fn similarity_aspect_strength_uses_query_relative_aspect_spread() {
        let mut rows = empty_similarity_aspect_score_rows(3);
        rows[0][SimilarityAspect::Spectrum.index()] = Some(1.0);
        rows[1][SimilarityAspect::Spectrum.index()] = Some(0.5);
        rows[2][SimilarityAspect::Spectrum.index()] = Some(-0.5);
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.7, 0.4],
            aspect_scores: rows,
            anchor_index: Some(0),
        };

        assert_eq!(
            query.aspect_score_for_index(SimilarityAspect::Spectrum, 1),
            Some(0.5)
        );
        assert_eq!(
            query.aspect_display_strength_for_index(SimilarityAspect::Spectrum, 0),
            Some(1.0)
        );
        assert!(
            query
                .aspect_display_strength_for_index(SimilarityAspect::Spectrum, 1)
                .is_some_and(|strength| strength > 0.6 && strength < 0.7)
        );
        assert_eq!(
            query.aspect_display_strength_for_index(SimilarityAspect::Timbre, 1),
            None
        );
    }

    #[test]
    fn effective_similarity_score_uses_enabled_aspect_controls() {
        let mut rows = empty_similarity_aspect_score_rows(2);
        rows[0][SimilarityAspect::Spectrum.index()] = Some(0.1);
        rows[1][SimilarityAspect::Spectrum.index()] = Some(0.9);
        let query = SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor"),
            indices: vec![0, 1],
            scores: vec![0.95, 0.2],
            aspect_scores: rows,
            anchor_index: Some(0),
        };
        let mut controls = SimilarityAspectSettings::default();
        controls.set_weighting_enabled(true);
        controls.set_aspect_enabled(SimilarityAspect::Overall, false);
        controls.set_aspect_enabled(SimilarityAspect::Timbre, false);
        controls.set_aspect_enabled(SimilarityAspect::Pitch, false);
        controls.set_aspect_enabled(SimilarityAspect::Amplitude, false);

        assert_eq!(query.effective_score_for_index(0, &controls), Some(0.1));
        assert_eq!(query.effective_score_for_index(1, &controls), Some(0.9));
        assert_eq!(
            query.display_strength_for_index_with_controls(1, &controls),
            Some(1.0)
        );
    }

    #[test]
    /// Sidebar metadata facets should combine format and BPM checks.
    fn sidebar_filter_state_accepts_format_and_bpm_facets() {
        let mut filters = BrowserSidebarFilterState::default();
        assert!(filters.toggle(
            BrowserSidebarFilterOption::Format(BrowserFormatFacet::Wav),
            true
        ));
        assert!(filters.toggle(BrowserSidebarFilterOption::Bpm(BrowserBpmFacet::Mid), true));

        assert!(filters.accepts_path_and_bpm(Path::new("drums/kick.wav"), Some(120.0)));
        assert!(filters.accepts_path_and_bpm(Path::new("drums/kick.WAV"), Some(90.0)));
        assert!(!filters.accepts_path_and_bpm(Path::new("drums/kick.aiff"), Some(120.0)));
        assert!(!filters.accepts_path_and_bpm(Path::new("drums/kick.wav"), Some(140.0)));
        assert!(!filters.accepts_path_and_bpm(Path::new("drums/kick.wav"), None));
    }
}
