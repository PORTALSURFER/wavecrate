use super::policy::{RefreshPolicy, apply_mutation_effects, refresh_effects};
use super::*;
use std::collections::BTreeSet;

pub(crate) fn set_browser_filter(controller: &mut AppController, filter: TriageFlagFilter) {
    let changed = controller.ui.browser.search.filter != filter;
    if changed {
        controller.ui.browser.search.filter = filter;
    }
    apply_mutation_effects(
        controller,
        refresh_effects(changed, RefreshPolicy::TriageFilter),
    );
}

/// Toggle one sidebar metadata-facet option and refresh visible rows when it changes.
pub(crate) fn toggle_browser_sidebar_filter(
    controller: &mut AppController,
    option: BrowserSidebarFilterOption,
    additive: bool,
) {
    let changed = controller
        .ui
        .browser
        .search
        .sidebar_filters
        .toggle(option, additive);
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Clear one sidebar metadata-facet group and refresh visible rows when it changes.
pub(crate) fn clear_browser_sidebar_filter(
    controller: &mut AppController,
    facet: BrowserSidebarFilterFacet,
) {
    let changed = controller
        .ui
        .browser
        .search
        .sidebar_filters
        .clear_facet(facet);
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Cycle the browser tag-derived filename filter through off, positive, and negated states.
pub(crate) fn toggle_browser_tag_named_filter(controller: &mut AppController, invert: bool) {
    let next = controller.ui.browser.search.tag_named_filter.next(invert);
    let changed = controller.ui.browser.search.tag_named_filter != next;
    if changed {
        controller.ui.browser.search.tag_named_filter = next;
    }
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Update the browser rating filter selection.
pub(crate) fn set_browser_rating_filter(controller: &mut AppController, level: i8, additive: bool) {
    let changed = set_rating_filter(
        &mut controller.ui.browser.search.rating_filter,
        level,
        additive,
    );
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Update the browser playback-age filter selection.
pub(crate) fn set_browser_playback_age_filter(
    controller: &mut AppController,
    chip: PlaybackAgeFilterChip,
    additive: bool,
) {
    let changed = set_single_select_filter(
        &mut controller.ui.browser.search.playback_age_filter,
        chip,
        additive,
    );
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Invert one browser rating-filter chip into every other valid filter level.
pub(crate) fn invert_browser_rating_filter(controller: &mut AppController, level: i8) {
    let Some(levels) = inverted_browser_rating_filter_levels(level) else {
        return;
    };
    let changed =
        replace_or_clear_matching_filter(&mut controller.ui.browser.search.rating_filter, levels);
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Invert one browser playback-age chip into every other valid playback-age chip.
pub(crate) fn invert_browser_playback_age_filter(
    controller: &mut AppController,
    chip: PlaybackAgeFilterChip,
) {
    let changed = replace_or_clear_matching_filter(
        &mut controller.ui.browser.search.playback_age_filter,
        inverted_browser_playback_age_filter_chips(chip),
    );
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Clear all browser rating filters.
pub(crate) fn clear_browser_rating_filter(controller: &mut AppController) {
    let changed = clear_filter(&mut controller.ui.browser.search.rating_filter);
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

/// Clear all browser playback-age filters.
pub(crate) fn clear_browser_playback_age_filter(controller: &mut AppController) {
    let changed = clear_filter(&mut controller.ui.browser.search.playback_age_filter);
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

fn set_rating_filter(filter: &mut BTreeSet<i8>, level: i8, additive: bool) -> bool {
    if !(-3..=4).contains(&level) {
        return false;
    }
    set_single_select_filter(filter, level, additive)
}

fn set_single_select_filter<T>(filter: &mut BTreeSet<T>, value: T, additive: bool) -> bool
where
    T: Copy + Ord,
{
    if additive {
        toggle_filter_value(filter, value);
        return true;
    }
    if filter.len() == 1 && filter.contains(&value) {
        return false;
    }
    filter.clear();
    filter.insert(value);
    true
}

fn toggle_filter_value<T>(filter: &mut BTreeSet<T>, value: T)
where
    T: Ord,
{
    if filter.contains(&value) {
        filter.remove(&value);
    } else {
        filter.insert(value);
    }
}

fn replace_or_clear_matching_filter<T>(filter: &mut BTreeSet<T>, next: BTreeSet<T>) -> bool
where
    T: Ord,
{
    if *filter == next {
        return clear_filter(filter);
    }
    *filter = next;
    true
}

fn clear_filter<T>(filter: &mut BTreeSet<T>) -> bool {
    if filter.is_empty() {
        return false;
    }
    filter.clear();
    true
}

/// Return every valid rating-filter level except the clicked chip level.
fn inverted_browser_rating_filter_levels(level: i8) -> Option<BTreeSet<i8>> {
    const ALL_BROWSER_RATING_FILTER_LEVELS: [i8; 8] = [-3, -2, -1, 0, 1, 2, 3, 4];
    if !ALL_BROWSER_RATING_FILTER_LEVELS.contains(&level) {
        return None;
    }
    Some(
        ALL_BROWSER_RATING_FILTER_LEVELS
            .into_iter()
            .filter(|candidate| *candidate != level)
            .collect(),
    )
}

/// Return every valid playback-age filter chip except the clicked chip.
fn inverted_browser_playback_age_filter_chips(
    chip: PlaybackAgeFilterChip,
) -> BTreeSet<PlaybackAgeFilterChip> {
    browser_playback_age_filter_chips()
        .into_iter()
        .filter(|candidate| *candidate != chip)
        .collect()
}
