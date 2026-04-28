//! Browser toolbar layout and hit-testing helpers.

use super::super::super::*;

pub(in crate::gui::native_shell::state) fn browser_toolbar_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> BrowserToolbarLayout {
    let sections = resolve_browser_toolbar_surface_layout(
        layout.browser_toolbar,
        style.sizing,
        &browser_toolbar_surface_content(model),
    );
    BrowserToolbarLayout {
        rating_filter_chips: sections.rating_filter_chips,
        playback_age_filter_chips: sections.playback_age_filter_chips,
        marked_filter_chip: sections.marked_filter_chip,
        action_slots: sections.action_slots,
        search_field: sections.search_field,
        activity_chip: sections.activity_chip,
        sort_chip: sections.sort_chip,
        triage_chips: sections.triage_chips,
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_index(
    level: i8,
) -> Option<usize> {
    BROWSER_RATING_FILTER_LEVELS
        .iter()
        .position(|chip| *chip == level)
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_level_at_point(
    chips: [Rect; 8],
    point: Point,
) -> Option<i8> {
    chips
        .iter()
        .position(|rect| rect.width() > 1.0 && rect.contains(point))
        .map(|index| BROWSER_RATING_FILTER_LEVELS[index])
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_index(
    chip: crate::app::PlaybackAgeFilterChip,
) -> Option<usize> {
    BROWSER_PLAYBACK_AGE_FILTER_CHIPS
        .iter()
        .position(|candidate| *candidate == chip)
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_at_point(
    chips: [Rect; 3],
    point: Point,
) -> Option<crate::app::PlaybackAgeFilterChip> {
    chips
        .iter()
        .position(|rect| rect.width() > 1.0 && rect.contains(point))
        .map(|index| BROWSER_PLAYBACK_AGE_FILTER_CHIPS[index])
}

pub(in crate::gui::native_shell::state) fn browser_column_chips(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    browser_buttons: &[ActionButton],
) -> Vec<BrowserColumnChip> {
    let _ = (layout, style, model, browser_buttons);
    Vec::new()
}
