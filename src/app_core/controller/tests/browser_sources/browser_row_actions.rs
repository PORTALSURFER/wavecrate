use super::*;

#[test]
fn apply_native_browser_normalize_routes_to_hotkey_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::NormalizeFocusedBrowserSample);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Focus a sample to normalize it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn toggle_focused_browser_row_selection_action_preserves_focus_and_anchor() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![
        browser_test_sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        browser_test_sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(1);

    controller.apply_native_ui_action(NativeUiAction::ToggleFocusedBrowserRowSelection);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("two.wav")]
    );

    controller.apply_native_ui_action(NativeUiAction::ToggleFocusedBrowserRowSelection);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
}
