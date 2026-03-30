//! Browser search state, async dispatch policy, and UI-triggered mutations.

use super::*;

mod cache;
mod dispatch_policy;
mod mutations;

pub(crate) use cache::BrowserSearchCache;
#[cfg(test)]
pub(crate) use dispatch_policy::with_browser_async_pipeline_enabled_for_tests;
pub(crate) use mutations::{
    blur_browser_search, clear_browser_playback_age_filter, clear_browser_rating_filter,
    focus_browser_search, invert_browser_playback_age_filter, invert_browser_rating_filter,
    set_browser_filter, set_browser_playback_age_filter, set_browser_rating_filter,
    set_browser_search, set_browser_sort, toggle_browser_marked_filter,
};
