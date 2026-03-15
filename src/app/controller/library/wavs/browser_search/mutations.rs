//! UI-triggered browser search/filter/sort mutations.

use super::*;
use crate::app::state::SampleBrowserSort;

/// Refresh browser rows through the authoritative async worker or the retained sync path.
fn refresh_browser_search_results(controller: &mut AppController) {
    if controller.should_dispatch_browser_search_async() {
        controller.dispatch_search_job();
    } else {
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn set_browser_filter(controller: &mut AppController, filter: TriageFlagFilter) {
    if controller.ui.browser.filter != filter {
        controller.ui.browser.filter = filter;
        controller.mark_browser_search_projection_revision_dirty();
        refresh_browser_search_results(controller);
    }
}

/// Update the browser rating filter selection.
pub(crate) fn set_browser_rating_filter(controller: &mut AppController, level: i8, additive: bool) {
    if !(-3..=4).contains(&level) {
        return;
    }
    let mut changed = false;
    if additive {
        if controller.ui.browser.rating_filter.contains(&level) {
            controller.ui.browser.rating_filter.remove(&level);
        } else {
            controller.ui.browser.rating_filter.insert(level);
        }
        changed = true;
    } else if controller.ui.browser.rating_filter.len() != 1
        || !controller.ui.browser.rating_filter.contains(&level)
    {
        controller.ui.browser.rating_filter.clear();
        controller.ui.browser.rating_filter.insert(level);
        changed = true;
    }
    if changed {
        controller.mark_browser_search_projection_revision_dirty();
        refresh_browser_search_results(controller);
    }
}

/// Replace the active browser rating filter set and refresh visible rows when it changes.
fn replace_browser_rating_filter(
    controller: &mut AppController,
    levels: impl IntoIterator<Item = i8>,
) {
    let next_filter = levels
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if controller.ui.browser.rating_filter == next_filter {
        return;
    }
    controller.ui.browser.rating_filter = next_filter;
    controller.mark_browser_search_projection_revision_dirty();
    refresh_browser_search_results(controller);
}

/// Return every valid rating-filter level except the clicked chip level.
fn inverted_browser_rating_filter_levels(level: i8) -> Option<std::collections::BTreeSet<i8>> {
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

/// Invert one browser rating-filter chip into every other valid filter level.
pub(crate) fn invert_browser_rating_filter(controller: &mut AppController, level: i8) {
    let Some(levels) = inverted_browser_rating_filter_levels(level) else {
        return;
    };
    if controller.ui.browser.rating_filter == levels {
        clear_browser_rating_filter(controller);
    } else {
        replace_browser_rating_filter(controller, levels);
    }
}

/// Clear all browser rating filters.
pub(crate) fn clear_browser_rating_filter(controller: &mut AppController) {
    if controller.ui.browser.rating_filter.is_empty() {
        return;
    }
    controller.ui.browser.rating_filter.clear();
    controller.mark_browser_search_projection_revision_dirty();
    refresh_browser_search_results(controller);
}

pub(crate) fn set_browser_sort(controller: &mut AppController, sort: SampleBrowserSort) {
    if controller.ui.browser.sort != sort {
        controller.ui.browser.sort = sort;
        if sort != SampleBrowserSort::Similarity {
            controller.ui.browser.similarity_sort_follow_loaded = false;
        }
        controller.mark_browser_search_projection_revision_dirty();
        refresh_browser_search_results(controller);
    }
}

pub(crate) fn focus_browser_search(controller: &mut AppController) {
    controller.focus_browser_context();
    if controller.ui.browser.search_focus_requested {
        return;
    }
    controller.ui.browser.search_focus_requested = true;
    controller.mark_browser_search_projection_revision_dirty();
}

/// Clear browser-search focus while leaving the current query text intact.
pub(crate) fn blur_browser_search(controller: &mut AppController) {
    if !controller.ui.browser.search_focus_requested {
        return;
    }
    controller.ui.browser.search_focus_requested = false;
    controller.mark_browser_search_projection_revision_dirty();
}

pub(crate) fn set_browser_search(controller: &mut AppController, query: impl Into<String>) {
    let query = query.into();
    if controller.ui.browser.search_query == query {
        return;
    }
    controller.ui.browser.search_query = query;
    controller.mark_browser_search_projection_revision_dirty();
    controller.ui.browser.similar_query = None;
    controller.ui.browser.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.similarity_sort_follow_loaded = false;
    refresh_browser_search_results(controller);
}
