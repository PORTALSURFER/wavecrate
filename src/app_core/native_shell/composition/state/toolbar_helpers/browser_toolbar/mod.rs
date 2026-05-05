//! Browser toolbar layout and hover helpers.

mod buttons;
mod colors;
mod layout;

#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use buttons::browser_action_buttons;
#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use colors::{
    browser_marked_filter_chip_border, browser_marked_filter_chip_contains_point,
    browser_marked_filter_chip_fill, browser_marked_filter_chip_hover_border,
    browser_marked_filter_chip_hover_fill, browser_playback_age_filter_chip_border,
    browser_playback_age_filter_chip_fill, browser_playback_age_filter_chip_hover_border,
    browser_playback_age_filter_chip_hover_fill, browser_rating_filter_chip_border,
    browser_rating_filter_chip_fill, browser_rating_filter_chip_hover_border,
    browser_rating_filter_chip_hover_fill, browser_search_field_hover_border,
    browser_search_field_hover_fill, render_browser_playback_age_filter_chip_hover_overlay,
    render_browser_rating_filter_chip_hover_overlay, render_browser_search_field_hover_overlay,
};
#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use layout::{
    browser_column_chips, browser_playback_age_filter_chip_at_point,
    browser_playback_age_filter_chip_index, browser_rating_filter_chip_index,
    browser_rating_filter_level_at_point, browser_toolbar_layout,
};
