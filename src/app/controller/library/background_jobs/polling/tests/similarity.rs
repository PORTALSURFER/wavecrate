use super::*;
use crate::app::controller::jobs::{
    FocusedSimilarityPaths, FocusedSimilarityResult, LoadedSimilarityQueryResult,
};
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::app::state::{SampleBrowserSort, SimilarQuery};
use crate::sample_sources::Rating;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn focused_similarity_message_ignores_stale_result_then_applies_matching_highlight() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.pending_focused_similarity_query = Some(
        crate::app::controller::state::runtime::PendingFocusedSimilarityQuery {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
        },
    );

    controller.apply_background_job_message_for_tests(JobMessage::FocusedSimilarityLoaded(
        FocusedSimilarityResult {
            request_id: 8,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(Some(FocusedSimilarityPaths {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                paths: vec![PathBuf::from("two.wav")],
                scores: vec![0.98],
                anchor_index: Some(0),
            })),
        },
    ));

    assert!(
        controller
            .runtime
            .pending_focused_similarity_query
            .is_some()
    );
    assert!(controller.ui.browser.search.focused_similarity.is_none());

    controller.apply_background_job_message_for_tests(JobMessage::FocusedSimilarityLoaded(
        FocusedSimilarityResult {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(Some(FocusedSimilarityPaths {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                paths: vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")],
                scores: vec![0.99, 0.98],
                anchor_index: Some(0),
            })),
        },
    ));

    assert!(
        controller
            .runtime
            .pending_focused_similarity_query
            .is_none()
    );
    let highlight = controller
        .ui
        .browser
        .search
        .focused_similarity
        .as_ref()
        .expect("focused similarity");
    assert_eq!(highlight.indices, vec![1]);
    assert_eq!(highlight.scores, vec![0.98]);
    assert_eq!(highlight.anchor_index, Some(0));
}

#[test]
fn loaded_similarity_query_message_ignores_stale_result_then_applies_matching_query() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similarity_sort_follow_loaded = true;
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        bytes: Arc::from(Vec::<u8>::new()),
        duration_seconds: 1.0,
        sample_rate: 44_100,
    });
    controller.runtime.pending_loaded_similarity_query = Some(
        crate::app::controller::state::runtime::PendingLoadedSimilarityQuery {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
        },
    );

    controller.apply_background_job_message_for_tests(JobMessage::LoadedSimilarityQueryBuilt(
        LoadedSimilarityQueryResult {
            request_id: 8,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(SimilarQuery {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.8],
                anchor_index: Some(0),
            }),
        },
    ));

    assert!(controller.runtime.pending_loaded_similarity_query.is_some());
    assert!(controller.ui.browser.search.similar_query.is_none());

    controller.apply_background_job_message_for_tests(JobMessage::LoadedSimilarityQueryBuilt(
        LoadedSimilarityQueryResult {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(SimilarQuery {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.8],
                anchor_index: Some(0),
            }),
        },
    ));

    assert!(controller.runtime.pending_loaded_similarity_query.is_none());
    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("similarity query");
    assert_eq!(query.indices, vec![0, 1]);
    assert_eq!(query.anchor_index, Some(0));
}
