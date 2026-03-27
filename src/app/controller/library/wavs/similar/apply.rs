use super::*;
use crate::app::state::{SampleBrowserSort, SimilarQuery};

pub(crate) fn apply_similarity_query(controller: &mut AppController, query: SimilarQuery) {
    controller.runtime.pending_loaded_similarity_query = None;
    controller.ui.browser.search.similar_query = Some(query);
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similarity_sort_follow_loaded = false;
    controller.ui.browser.selection.autoscroll = true;
    controller.ui.browser.viewport.view_window_start = 0;
    controller.ui.browser.viewport.render_window_start = 0;
    if !controller.ui.browser.search.search_query.is_empty() {
        controller.mark_browser_search_projection_revision_dirty();
    }
    controller.ui.browser.search.search_query.clear();
    controller.ui.browser.search.search_focus_requested = false;
    controller.rebuild_browser_lists();
}

pub(crate) fn clear_similar_filter(controller: &mut AppController) {
    controller.runtime.pending_loaded_similarity_query = None;
    if controller.ui.browser.search.similar_query.take().is_some() {
        controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
        controller.ui.browser.search.similarity_sort_follow_loaded = false;
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn disable_similarity_sort(controller: &mut AppController) {
    controller.runtime.pending_loaded_similarity_query = None;
    controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.search.similarity_sort_follow_loaded = false;
    controller.ui.browser.search.similar_query = None;
    controller.rebuild_browser_lists();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;

    #[test]
    fn apply_similarity_query_resets_browser_state_and_preserves_anchor() {
        let (mut controller, _source) = dummy_controller();
        controller.ui.browser.search.search_query = "query".to_string();
        controller.ui.browser.search.search_focus_requested = true;
        controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
        controller.ui.browser.search.similarity_sort_follow_loaded = true;
        controller.ui.browser.selection.autoscroll = false;
        controller.ui.browser.viewport.view_window_start = 17;
        controller.ui.browser.viewport.render_window_start = 9;
        let query = SimilarQuery {
            sample_id: "sample-id".to_string(),
            label: "Sample".to_string(),
            indices: vec![0],
            scores: vec![0.5],
            anchor_index: Some(2),
        };
        apply_similarity_query(&mut controller, query);
        let applied = controller.ui.browser.search.similar_query.as_ref().unwrap();
        assert_eq!(
            controller.ui.browser.search.sort,
            SampleBrowserSort::Similarity
        );
        assert!(!controller.ui.browser.search.similarity_sort_follow_loaded);
        assert!(controller.ui.browser.search.search_query.is_empty());
        assert!(!controller.ui.browser.search.search_focus_requested);
        assert!(controller.ui.browser.selection.autoscroll);
        assert_eq!(controller.ui.browser.viewport.view_window_start, 0);
        assert_eq!(controller.ui.browser.viewport.render_window_start, 0);
        assert_eq!(applied.anchor_index, Some(2));
    }
}
