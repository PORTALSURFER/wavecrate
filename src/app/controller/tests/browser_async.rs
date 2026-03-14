use super::super::jobs::{JobMessage, SearchResult};
use super::super::library::wavs::with_browser_async_pipeline_enabled_for_tests;
use super::super::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use super::common::visible_indices;
use crate::app::state::VisibleRows;
use crate::sample_sources::Rating;
use std::sync::Arc;

#[test]
fn async_browser_search_dispatches_then_applies_matching_result() {
    let entries = vec![
        sample_entry("zzabc.wav", Rating::NEUTRAL),
        sample_entry("abc_extra.wav", Rating::NEUTRAL),
        sample_entry("abc.wav", Rating::NEUTRAL),
    ];
    let (mut sync_controller, _sync_source) = prepare_with_source_and_wav_entries(entries.clone());
    sync_controller.set_browser_search("abc");
    let expected_visible = visible_indices(&sync_controller);
    let expected_scores = sync_controller.ui_cache.browser.search.scores.clone();
    let expected_trash = Arc::clone(&sync_controller.ui.browser.trash);
    let expected_neutral = Arc::clone(&sync_controller.ui.browser.neutral);
    let expected_keep = Arc::clone(&sync_controller.ui.browser.keep);

    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    let starting_visible = visible_indices(&controller);

    with_browser_async_pipeline_enabled_for_tests(true, || {
        controller.set_browser_search("abc");

        let request_id = controller.ui.browser.latest_search_request_id;
        assert_eq!(request_id, 1);
        assert!(controller.ui.browser.search_busy);
        assert_eq!(controller.ui.browser.latest_applied_search_request_id, 0);
        assert_eq!(visible_indices(&controller), starting_visible);

        controller.apply_background_job_message_for_tests(JobMessage::BrowserSearchFinished(
            SearchResult {
                request_id,
                source_id: source.id.clone(),
                query: "abc".into(),
                visible: VisibleRows::List(expected_visible.clone().into()),
                trash: expected_trash,
                neutral: expected_neutral,
                keep: expected_keep,
                scores: expected_scores,
            },
        ));
    });

    assert_eq!(visible_indices(&controller), expected_visible);
    assert_eq!(controller.ui.browser.latest_applied_search_request_id, 1);
    assert!(!controller.ui.browser.search_busy);
}

#[test]
fn stale_async_browser_search_result_is_ignored_until_latest_request_arrives() {
    let entries = vec![
        sample_entry("kick.wav", Rating::NEUTRAL),
        sample_entry("snare.wav", Rating::NEUTRAL),
        sample_entry("hat.wav", Rating::NEUTRAL),
    ];
    let (mut sync_controller, _sync_source) = prepare_with_source_and_wav_entries(entries.clone());
    sync_controller.set_browser_search("snr");
    let expected_visible = visible_indices(&sync_controller);
    let expected_scores = sync_controller.ui_cache.browser.search.scores.clone();

    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    let starting_visible = visible_indices(&controller);

    with_browser_async_pipeline_enabled_for_tests(true, || {
        controller.set_browser_search("kick");
        let first_request_id = controller.ui.browser.latest_search_request_id;
        controller.set_browser_search("snr");
        let second_request_id = controller.ui.browser.latest_search_request_id;

        assert_eq!(first_request_id, 1);
        assert_eq!(second_request_id, 2);
        assert!(controller.ui.browser.search_busy);

        controller.apply_background_job_message_for_tests(JobMessage::BrowserSearchFinished(
            SearchResult {
                request_id: first_request_id,
                source_id: source.id.clone(),
                query: "kick".into(),
                visible: VisibleRows::List(Arc::from([0usize])),
                trash: Arc::from([]),
                neutral: Arc::from([0usize]),
                keep: Arc::from([]),
                scores: Arc::from([Some(99_i64), None, None]),
            },
        ));

        assert_eq!(visible_indices(&controller), starting_visible);
        assert_eq!(controller.ui.browser.latest_applied_search_request_id, 0);
        assert!(controller.ui.browser.search_busy);

        controller.apply_background_job_message_for_tests(JobMessage::BrowserSearchFinished(
            SearchResult {
                request_id: second_request_id,
                source_id: source.id.clone(),
                query: "snr".into(),
                visible: VisibleRows::List(expected_visible.clone().into()),
                trash: Arc::from([]),
                neutral: Arc::from([0usize, 1usize, 2usize]),
                keep: Arc::from([]),
                scores: expected_scores,
            },
        ));
    });

    assert_eq!(visible_indices(&controller), expected_visible);
    assert_eq!(controller.ui.browser.latest_applied_search_request_id, 2);
    assert!(!controller.ui.browser.search_busy);
}
