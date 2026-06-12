use super::super::*;
use crate::app::state::{BrowserSidebarFilterFacet, BrowserSidebarFilterOption};

impl AppController {
    /// Apply a new browser filter and refresh visible rows.
    pub fn set_browser_filter(&mut self, filter: TriageFlagFilter) {
        browser_search::set_browser_filter(self, filter);
    }

    /// Apply a rating-level filter to the browser list (`-3..=3`, plus `4` for locked keeps).
    pub fn set_browser_rating_filter(&mut self, level: i8, additive: bool) {
        browser_search::set_browser_rating_filter(self, level, additive);
    }

    /// Apply a playback-age chip filter to the browser list.
    pub fn set_browser_playback_age_filter(
        &mut self,
        chip: crate::app::state::PlaybackAgeFilterChip,
        additive: bool,
    ) {
        browser_search::set_browser_playback_age_filter(self, chip, additive);
    }

    /// Invert one rating chip into the opposite rated bucket in the browser list.
    pub fn invert_browser_rating_filter(&mut self, level: i8) {
        browser_search::invert_browser_rating_filter(self, level);
    }

    /// Invert one playback-age chip into the opposite playback-age buckets in the browser list.
    pub fn invert_browser_playback_age_filter(
        &mut self,
        chip: crate::app::state::PlaybackAgeFilterChip,
    ) {
        browser_search::invert_browser_playback_age_filter(self, chip);
    }

    /// Clear any active rating-level filters in the browser list.
    pub fn clear_browser_rating_filter(&mut self) {
        browser_search::clear_browser_rating_filter(self);
    }

    /// Clear any active playback-age filters in the browser list.
    pub fn clear_browser_playback_age_filter(&mut self) {
        browser_search::clear_browser_playback_age_filter(self);
    }

    /// Toggle one sidebar metadata-facet option in the browser list.
    pub fn toggle_browser_sidebar_filter(
        &mut self,
        option: BrowserSidebarFilterOption,
        additive: bool,
    ) {
        browser_search::toggle_browser_sidebar_filter(self, option, additive);
    }

    /// Clear one sidebar metadata-facet group in the browser list.
    pub fn clear_browser_sidebar_filter(&mut self, facet: BrowserSidebarFilterFacet) {
        browser_search::clear_browser_sidebar_filter(self, facet);
    }

    /// Toggle whether the browser shows only session-marked samples.
    pub fn toggle_browser_marked_filter(&mut self) {
        self.toggle_browser_marked_filter_action();
    }

    /// Cycle the browser tag-derived filename filter.
    pub fn toggle_browser_tag_named_filter(&mut self, invert: bool) {
        browser_search::toggle_browser_tag_named_filter(self, invert);
    }

    /// Toggle the session mark for the focused row or current multi-selection.
    pub fn toggle_browser_sample_mark(&mut self) {
        self.toggle_browser_sample_mark_action();
    }

    /// Apply a new sample browser sort mode and refresh visible rows.
    pub fn set_browser_sort(&mut self, sort: SampleBrowserSort) {
        browser_search::set_browser_sort(self, sort);
    }

    /// Request focus for the browser search input while keeping the browser context active.
    pub(crate) fn focus_browser_search(&mut self) {
        browser_search::focus_browser_search(self);
    }

    /// Clear browser-search focus while preserving the current query text.
    pub(crate) fn blur_browser_search(&mut self) {
        browser_search::blur_browser_search(self);
    }

    /// Apply a fuzzy search query to the browser and refresh visible rows.
    pub fn set_browser_search(&mut self, query: impl Into<String>) {
        browser_search::set_browser_search(self, query);
    }
}
