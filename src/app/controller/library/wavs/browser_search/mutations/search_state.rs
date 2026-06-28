use super::policy::{RefreshPolicy, apply_mutation_effects, refresh_effects};
use super::*;

pub(crate) fn set_browser_sort(controller: &mut AppController, sort: SampleBrowserSort) {
    let changed = controller.ui.browser.search.sort != sort;
    if changed {
        controller.ui.browser.search.sort = sort;
        if sort != SampleBrowserSort::Similarity {
            controller.ui.browser.search.similarity_sort_follow_loaded = false;
        }
    }
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}

pub(crate) fn set_browser_search(controller: &mut AppController, query: impl Into<String>) {
    let query = query.into();
    let changed = controller.ui.browser.search.search_query != query;
    if changed {
        controller.ui.browser.search.search_query = query;
        controller.ui.browser.search.similar_query = None;
        controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
        controller.ui.browser.search.similarity_sort_follow_loaded = false;
    }
    apply_mutation_effects(controller, refresh_effects(changed, RefreshPolicy::Full));
}
