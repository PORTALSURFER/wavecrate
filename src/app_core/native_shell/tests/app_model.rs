use super::*;

/// Build a controller fixture with non-default fields for full app-model parity checks.
fn app_model_projection_fixture_controller() -> AppController {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.status.text = String::from("Projection fixture status");
    controller.ui.volume = 1.25;
    controller.ui.browser.visible = crate::app_core::app_api::state::VisibleRows::All { total: 24 };
    controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeAsc;
    controller.ui.browser.search_query = String::from("kick");
    controller.ui.browser.search_busy = true;
    controller.ui.browser.selected = Some(crate::app_core::app_api::state::SampleBrowserIndex {
        column: TriageFlagColumn::Keep,
        row: 0,
    });
    controller.ui.browser.active_tab = SampleBrowserTab::List;
    controller.ui.waveform.loop_enabled = true;
    controller.ui.update.status = UpdateStatus::Checking;
    controller
}

#[test]
/// Staged projection helpers should assemble the same app model as `project_app_model`.
fn project_app_model_matches_staged_projection_helpers() {
    let mut expected_controller = app_model_projection_fixture_controller();
    let derived_inputs = derive_project_app_model_inputs(&expected_controller);
    let core_models = materialize_project_app_model_core(&mut expected_controller, &derived_inputs);
    let overlay_and_chrome_models = materialize_project_app_model_overlay_and_chrome(
        &expected_controller.ui,
        core_models.browser.visible_count,
    );
    let expected =
        assemble_project_app_model(derived_inputs, core_models, overlay_and_chrome_models);

    let mut actual_controller = app_model_projection_fixture_controller();
    let actual = project_app_model(&mut actual_controller);

    assert_eq!(actual, expected);
}

/// Update projection should expose the status text and action hints for each update state.
#[test]
fn update_projection_exposes_status_and_action_hint_labels() {
    let mut ui = UiState::default();
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Idle);
    assert_eq!(projected.status_label, "Updates: idle");
    assert_eq!(projected.action_hint_label, "Action: check");
    assert!(projected.release_notes_label.is_empty());

    ui.update.status = UpdateStatus::UpdateAvailable;
    ui.update.available_tag = Some(String::from("v20.1.0"));
    ui.update.available_url = Some(String::from("https://example.invalid/release"));
    ui.update.available_published_at = Some(String::from("2026-02-01T12:00:00Z"));
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Available);
    assert_eq!(
        projected.status_label,
        "Update available: v20.1.0 (manual install required)"
    );
    assert_eq!(
        projected.action_hint_label,
        "Actions: open | install(manual) | dismiss"
    );
    assert_eq!(
        projected.release_notes_label,
        "Release: v20.1.0 (2026-02-01T12:00:00Z) | Signed manual install required"
    );

    ui.update.status = UpdateStatus::Error;
    ui.update.last_error = Some(String::from("network timeout"));
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Error);
    assert_eq!(
        projected.status_label,
        "Update check failed: network timeout"
    );
    assert_eq!(projected.action_hint_label, "Action: retry");
    assert!(projected.release_notes_label.is_empty());
}
