//! UI-triggered browser search/filter/sort mutations.

use super::*;
use crate::app::state::{
    BrowserSidebarFilterFacet, BrowserSidebarFilterOption, PlaybackAgeFilterChip,
    SampleBrowserSort, browser_playback_age_filter_chips,
};

mod filter_state;
mod policy;
mod search_state;

pub(crate) use filter_state::{
    clear_browser_playback_age_filter, clear_browser_rating_filter, clear_browser_sidebar_filter,
    invert_browser_playback_age_filter, invert_browser_rating_filter, set_browser_filter,
    set_browser_playback_age_filter, set_browser_rating_filter, toggle_browser_sidebar_filter,
    toggle_browser_tag_named_filter,
};
pub(crate) use search_state::{
    blur_browser_search, focus_browser_search, set_browser_search, set_browser_sort,
};
