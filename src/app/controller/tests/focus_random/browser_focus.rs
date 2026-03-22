use super::*;

#[test]
fn focusing_browser_row_updates_focus_context() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "focus.wav");
    controller.focus_browser_row(0);
    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
}

#[test]
fn hotkey_search_browser_requests_focus() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "find.wav");
    controller.ui.browser.search.search_focus_requested = false;
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "search-browser")
        .expect("search-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert!(controller.ui.browser.search.search_focus_requested);
    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
}

#[test]
fn focusing_browser_list_clears_search_focus_request() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "find.wav");
    controller.focus_browser_search();

    controller.focus_browser_list();

    assert!(!controller.ui.browser.search.search_focus_requested);
    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
}

#[test]
fn find_similar_hotkey_is_registered() {
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "find-similar")
        .expect("find-similar hotkey");
    assert_eq!(action.label, "Toggle find similar");
    assert_eq!(
        action.scope,
        hotkeys::HotkeyScope::Focus(FocusContext::SampleBrowser)
    );
    assert_eq!(action.gesture.first.key, KeyCode::F);
    assert!(action.gesture.first.shift);
    assert!(!action.gesture.first.command);
    assert!(!action.gesture.first.alt);
    assert!(action.gesture.chord.is_none());
}

#[test]
fn find_similar_from_map_switches_to_browser_list() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "map.wav");
    controller.focus_browser_row(0);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    controller.ui.browser.search.similar_query = Some(crate::app::state::SimilarQuery {
        sample_id: "test::map.wav".to_string(),
        label: "map.wav".to_string(),
        indices: vec![0],
        scores: vec![1.0],
        anchor_index: Some(0),
    });
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "find-similar")
        .expect("find-similar hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert_eq!(controller.ui.browser.active_tab, SampleBrowserTab::List);
    assert!(controller.ui.browser.search.similar_query.is_none());
}

#[test]
fn hotkey_focus_waveform_sets_context() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "wave.wav");
    controller.select_wav_by_path(Path::new("wave.wav"));
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "focus-waveform")
        .expect("focus-waveform hotkey");
    controller.handle_hotkey(action, FocusContext::None);
    assert_eq!(controller.ui.focus.context, FocusContext::Waveform);
}

#[test]
fn hotkey_toggle_selection_dispatches_in_browser_context() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "toggle.wav");
    controller.focus_browser_row(0);
    assert_eq!(controller.ui.browser.selection.selected_paths.len(), 1);
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "toggle-select")
        .expect("toggle-select hotkey");
    controller.handle_hotkey(action, FocusContext::SampleBrowser);
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
}
