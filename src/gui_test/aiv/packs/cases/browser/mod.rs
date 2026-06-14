mod filters;
mod map_focus;
mod search;
mod viewport_scroll;

pub(crate) use filters::{browser_playback_age_filters_case, browser_tabs_and_rating_filters_case};
pub(crate) use map_focus::browser_map_point_focus_case;
pub(crate) use search::{browser_search_select_commit_case, browser_search_type_smoke_case};
pub(crate) use viewport_scroll::{
    browser_interior_click_keeps_viewport_after_down_scroll_case,
    browser_interior_click_keeps_viewport_after_up_scroll_case,
    browser_refocus_after_down_scroll_keeps_single_focus_case,
    browser_repeated_scroll_refocus_preserves_guard_band_case,
    browser_wheel_scroll_uses_rendered_viewport_case,
};
