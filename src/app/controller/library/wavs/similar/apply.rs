use super::*;
use crate::app::state::{SampleBrowserSort, SimilarQuery};
use crate::app_core::state::StatusTone;
use std::collections::HashSet;
use std::path::PathBuf;

pub(crate) fn apply_similarity_query(controller: &mut AppController, query: SimilarQuery) {
    cancel_pending_similarity_filter_rebuild(controller);
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
    cancel_pending_similarity_filter_rebuild(controller);
    controller.runtime.pending_loaded_similarity_query = None;
    if controller.ui.browser.search.similar_query.take().is_some() {
        controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
        controller.ui.browser.search.similarity_sort_follow_loaded = false;
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn disable_similarity_sort(controller: &mut AppController) {
    cancel_pending_similarity_filter_rebuild(controller);
    controller.runtime.pending_loaded_similarity_query = None;
    controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.search.similarity_sort_follow_loaded = false;
    controller.ui.browser.search.similar_query = None;
    controller.rebuild_browser_lists();
}

pub(crate) fn cancel_pending_similarity_filter_rebuild(controller: &mut AppController) {
    controller.runtime.pending_similarity_filter_rebuild = None;
}

pub(crate) fn schedule_similarity_filter_rebuild_after_delete(
    controller: &mut AppController,
    deleted_paths: &HashSet<PathBuf>,
) {
    cancel_pending_similarity_filter_rebuild(controller);
    let Some(selected_source_id) = controller.selected_source_id() else {
        clear_manual_similarity_filter_state_without_rebuild(controller);
        return;
    };
    let Some(query) = controller.ui.browser.search.similar_query.clone() else {
        return;
    };
    if controller.ui.browser.search.similarity_sort_follow_loaded {
        return;
    }

    let next_anchor_path = next_similarity_anchor_after_delete(controller, &query, deleted_paths);
    clear_manual_similarity_filter_state_without_rebuild(controller);
    if let Some(anchor_relative_path) = next_anchor_path {
        controller.runtime.pending_similarity_filter_rebuild = Some(
            crate::app::controller::state::runtime::PendingSimilarityFilterRebuild {
                source_id: selected_source_id,
                anchor_relative_path,
            },
        );
    }
}

pub(crate) fn apply_pending_similarity_filter_rebuild(controller: &mut AppController) {
    let Some(pending) = controller.runtime.pending_similarity_filter_rebuild.clone() else {
        return;
    };
    if controller.selected_source_id().as_ref() != Some(&pending.source_id) {
        cancel_pending_similarity_filter_rebuild(controller);
        return;
    }
    if controller
        .wav_index_for_path(&pending.anchor_relative_path)
        .is_none()
    {
        cancel_pending_similarity_filter_rebuild(controller);
        return;
    }

    cancel_pending_similarity_filter_rebuild(controller);
    let sample_id = super::analysis_jobs::build_sample_id(
        pending.source_id.as_str(),
        &pending.anchor_relative_path,
    );
    if let Err(err) = super::find_similar_for_sample_id(controller, &sample_id) {
        controller.set_status(
            format!("Find similar failed after delete: {err}"),
            StatusTone::Warning,
        );
    }
}

fn clear_manual_similarity_filter_state_without_rebuild(controller: &mut AppController) {
    controller.runtime.pending_loaded_similarity_query = None;
    controller.ui.browser.search.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.search.similarity_sort_follow_loaded = false;
    controller.ui.browser.search.similar_query = None;
}

fn next_similarity_anchor_after_delete(
    controller: &mut AppController,
    query: &SimilarQuery,
    deleted_paths: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let current_anchor_path = query_anchor_path(controller, query);
    if let Some(anchor_path) = current_anchor_path.as_ref()
        && !deleted_paths.contains(anchor_path)
    {
        return Some(anchor_path.clone());
    }

    query
        .indices
        .iter()
        .copied()
        .filter_map(|index| {
            controller
                .wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        })
        .find(|path| !deleted_paths.contains(path))
}

fn query_anchor_path(controller: &mut AppController, query: &SimilarQuery) -> Option<PathBuf> {
    parse_query_anchor_path(controller.selected_source_id().as_ref(), &query.sample_id).or_else(
        || {
            query.anchor_index.and_then(|index| {
                controller
                    .wav_entry(index)
                    .map(|entry| entry.relative_path.clone())
            })
        },
    )
}

fn parse_query_anchor_path(
    selected_source_id: Option<&SourceId>,
    sample_id: &str,
) -> Option<PathBuf> {
    let (source_id, relative_path) = super::analysis_jobs::parse_sample_id(sample_id).ok()?;
    selected_source_id
        .filter(|selected_source_id| selected_source_id.as_str() == source_id.as_str())
        .map(|_| relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;
    use crate::sample_sources::SourceId;
    use std::path::Path;

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

    #[test]
    fn next_similarity_anchor_after_delete_promotes_next_survivor_when_anchor_is_deleted() {
        let (mut controller, _source) = dummy_controller();
        controller.set_wav_entries_for_tests(vec![
            crate::app::controller::test_support::sample_entry(
                "a.wav",
                crate::sample_sources::Rating::NEUTRAL,
            ),
            crate::app::controller::test_support::sample_entry(
                "b.wav",
                crate::sample_sources::Rating::NEUTRAL,
            ),
            crate::app::controller::test_support::sample_entry(
                "c.wav",
                crate::sample_sources::Rating::NEUTRAL,
            ),
        ]);
        let query = SimilarQuery {
            sample_id: "source::a.wav".to_string(),
            label: "a.wav".to_string(),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.9, 0.8],
            anchor_index: Some(0),
        };
        let deleted_paths = HashSet::from([PathBuf::from("a.wav")]);

        let next_anchor =
            next_similarity_anchor_after_delete(&mut controller, &query, &deleted_paths);

        assert_eq!(next_anchor.as_deref(), Some(Path::new("b.wav")));
    }

    #[test]
    fn parse_query_anchor_path_requires_matching_selected_source() {
        let selected_source_id = SourceId::from_string("source-a");

        let matches = parse_query_anchor_path(Some(&selected_source_id), "source-a::anchor.wav");
        let other = parse_query_anchor_path(Some(&selected_source_id), "source-b::anchor.wav");

        assert_eq!(matches.as_deref(), Some(Path::new("anchor.wav")));
        assert!(other.is_none());
    }
}
