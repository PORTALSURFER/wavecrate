use super::*;
use crate::app::state::{IssueTokenStatus, ProgressOverlayState, ProgressTaskKind, SimilarQuery};
use crate::app_core::controller::build_named_gui_fixture_controller;
use std::path::Path;

fn with_fixture_controller(tag: &str, run: impl FnOnce(&mut AppController)) {
    let mut bundle = build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), tag)
        .unwrap_or_else(|err| panic!("failed to build {tag} fixture: {err}"));
    run(&mut bundle.controller);
}

fn dummy_similar_query() -> SimilarQuery {
    SimilarQuery {
        sample_id: String::from("fixture::kick_one.wav"),
        label: String::from("kick_one.wav"),
        indices: vec![0, 1],
        scores: vec![1.0, 0.95],
        anchor_index: Some(0),
    }
}

#[test]
fn apply_native_commit_focused_browser_row_uses_browser_commit_path_when_browser_has_focus() {
    with_fixture_controller("browser", |controller| {
        controller.focus_browser_row_only(1);

        controller.apply_native_ui_action(NativeUiAction::CommitFocusedBrowserRow);

        assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
        assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
        assert_eq!(
            controller.focused_browser_path().as_deref(),
            Some(Path::new("snare_two.wav"))
        );
        assert!(
            !controller.ui.status.text.contains("Audio unavailable"),
            "status was {:?}",
            controller.ui.status.text
        );
    });
}

#[test]
fn apply_native_commit_focused_browser_row_falls_back_to_transport_outside_browser_focus() {
    with_fixture_controller("browser", |controller| {
        controller.ui.focus.context = FocusContext::Waveform;

        controller.apply_native_ui_action(NativeUiAction::CommitFocusedBrowserRow);

        assert!(
            controller.ui.status.text.contains("Audio"),
            "status was {:?}",
            controller.ui.status.text
        );
    });
}

#[test]
fn apply_native_toggle_find_similar_switches_map_to_list_before_clearing_query() {
    with_fixture_controller("map", |controller| {
        controller.focus_browser_row_only(0);
        controller.ui.browser.search.similar_query = Some(SimilarQuery {
            sample_id: controller
                .sample_id_for_visible_row(0)
                .expect("focused row sample id"),
            ..dummy_similar_query()
        });

        controller.apply_native_ui_action(NativeUiAction::ToggleFindSimilarFocusedSample);

        assert_eq!(controller.ui.browser.active_tab, SampleBrowserTab::List);
        assert!(controller.ui.browser.search.similar_query.is_none());
    });
}

#[test]
fn apply_native_toggle_find_similar_clears_existing_query() {
    with_fixture_controller("browser", |controller| {
        controller.focus_browser_row_only(0);
        controller.ui.browser.search.similar_query = Some(SimilarQuery {
            sample_id: controller
                .sample_id_for_visible_row(0)
                .expect("focused row sample id"),
            ..dummy_similar_query()
        });

        controller.apply_native_ui_action(NativeUiAction::ToggleFindSimilarFocusedSample);

        assert!(controller.ui.browser.search.similar_query.is_none());
    });
}

#[test]
fn apply_native_toggle_find_similar_without_focus_sets_status() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::ToggleFindSimilarFocusedSample);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Focus a sample to find similar"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_focus_map_sample_stages_selection_and_preview() {
    with_fixture_controller("map", |controller| {
        let sample_id = controller.ui.map.cached_points[0].sample_id.to_string();

        controller.apply_native_ui_action(NativeUiAction::FocusMapSample {
            sample_id: sample_id.clone(),
        });

        assert_eq!(controller.ui.browser.active_tab, SampleBrowserTab::Map);
        assert_eq!(
            controller.ui.map.selected_sample_id.as_deref(),
            Some(sample_id.as_str())
        );
        assert_eq!(
            controller.ui.map.hovered_sample_id.as_deref(),
            Some(sample_id.as_str())
        );
        assert_eq!(
            controller.focused_browser_path().as_deref(),
            Some(Path::new("kick_one.wav"))
        );
    });
}

#[test]
fn apply_native_cancel_progress_only_sets_cancel_flag_for_cancelable_tasks() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.progress =
        ProgressOverlayState::new(ProgressTaskKind::SelectionExport, "Export", 2, true);

    controller.apply_native_ui_action(NativeUiAction::CancelProgress);

    assert!(controller.ui.progress.cancel_requested);

    controller.ui.progress = ProgressOverlayState::new(ProgressTaskKind::Scan, "Scan", 4, false);
    controller.apply_native_ui_action(NativeUiAction::CancelProgress);

    assert!(!controller.ui.progress.cancel_requested);
}

#[test]
fn apply_native_open_feedback_issue_prompt_closes_overlay_and_primes_issue_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.hotkeys.overlay_visible = true;
    controller.ui.feedback_issue.token_status = IssueTokenStatus::Connected;
    controller.ui.feedback_issue.token_cached = Some(String::from("cached-token"));
    controller.ui.feedback_issue.last_error = Some(String::from("stale error"));
    controller.ui.feedback_issue.last_success_url = Some(String::from("https://example.invalid"));

    controller.apply_native_ui_action(NativeUiAction::OpenFeedbackIssuePrompt);

    assert!(!controller.ui.hotkeys.overlay_visible);
    assert!(controller.ui.feedback_issue.open);
    assert!(controller.ui.feedback_issue.focus_title_requested);
    assert_eq!(
        controller.ui.feedback_issue.token_status,
        IssueTokenStatus::Unknown
    );
    assert_eq!(controller.ui.feedback_issue.token_cached, None);
    assert!(controller.ui.feedback_issue.token_loading);
    assert!(controller.ui.feedback_issue.last_error.is_none());
    assert!(controller.ui.feedback_issue.last_success_url.is_none());
}

#[test]
fn apply_native_open_update_link_without_url_is_noop() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.update.status = UpdateStatus::Error;
    controller.ui.update.last_error = Some(String::from("existing error"));

    controller.apply_native_ui_action(NativeUiAction::OpenUpdateLink);

    assert_eq!(controller.ui.update.status, UpdateStatus::Error);
    assert_eq!(
        controller.ui.update.last_error.as_deref(),
        Some("existing error")
    );
}

#[test]
fn apply_native_install_update_without_available_release_sets_info_status() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::InstallUpdate);

    assert_eq!(controller.ui.update.status, UpdateStatus::Idle);
    assert_eq!(controller.ui.status.text, "No update available");
}

#[test]
fn apply_native_dismiss_update_clears_available_release_state() {
    with_fixture_controller("update", |controller| {
        controller.apply_native_ui_action(NativeUiAction::DismissUpdate);

        assert_eq!(controller.ui.update.status, UpdateStatus::Idle);
        assert!(controller.ui.update.available_tag.is_none());
        assert!(controller.ui.update.available_url.is_none());
        assert!(controller.ui.update.available_published_at.is_none());
        assert_eq!(
            controller
                .ui
                .update
                .last_seen_nightly_published_at
                .as_deref(),
            Some("2026-03-11T12:00:00Z")
        );
    });
}
