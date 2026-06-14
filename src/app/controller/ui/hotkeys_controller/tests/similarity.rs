use super::*;
use crate::app_core::controller::build_named_gui_fixture_controller;
use crate::waveform::WaveformRenderer;

#[test]
fn browser_similarity_hotkey_matches_native_toggle_behavior_when_focus_changes() {
    let action = action_for(|action| {
        matches!(
            action,
            crate::app_core::actions::NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleFindSimilarFocusedSample
            )
        )
    });
    let mut hotkey_bundle =
        build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "browser")
            .expect("browser fixture");
    let mut native_bundle =
        build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "browser")
            .expect("browser fixture");

    seed_similarity_query_with_different_focus(&mut hotkey_bundle.controller);
    seed_similarity_query_with_different_focus(&mut native_bundle.controller);

    hotkey_bundle
        .controller
        .handle_hotkey(action.clone(), FocusContext::SampleBrowser);
    native_bundle.controller.apply_ui_action(action.action);

    let hotkey_query = hotkey_bundle
        .controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .map(|query| sample_name(&query.sample_id).to_string());
    let native_query = native_bundle
        .controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .map(|query| sample_name(&query.sample_id).to_string());

    assert!(
        hotkey_query.is_some(),
        "hotkey should not clear a mismatched anchor"
    );
    assert_eq!(hotkey_query, native_query);
    assert_eq!(
        hotkey_bundle.controller.ui.browser.active_tab,
        native_bundle.controller.ui.browser.active_tab
    );
}
