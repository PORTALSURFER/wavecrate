use super::*;

#[test]
fn click_clears_selection_and_focuses_row() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(1);
    assert_eq!(controller.ui.browser.selection.selected_paths.len(), 2);

    controller.clear_browser_selection();
    controller.focus_browser_row_only(2);

    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(2)
    );
}

#[test]
fn ctrl_click_toggles_selection_and_focuses_row() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );

    controller.toggle_browser_row_selection(2);

    let selected: Vec<_> = controller.ui.browser.selection.selected_paths.to_vec();
    assert!(selected.contains(&PathBuf::from("one.wav")));
    assert!(selected.contains(&PathBuf::from("three.wav")));
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}

#[test]
fn shift_click_extends_selection_range() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(2);

    controller.extend_browser_selection_to_row(1);

    let selected: Vec<_> = controller.ui.browser.selection.selected_paths.to_vec();
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&PathBuf::from("one.wav")));
    assert!(selected.contains(&PathBuf::from("two.wav")));
    assert!(!selected.contains(&PathBuf::from("three.wav")));
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}

#[test]
fn ctrl_shift_click_adds_range_without_resetting_anchor() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("five.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("six.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(5);

    controller.add_range_browser_selection(2);

    let selected: Vec<_> = controller
        .ui
        .browser
        .selection
        .selected_paths
        .iter()
        .cloned()
        .collect();
    assert_eq!(selected.len(), 4);
    assert!(selected.contains(&PathBuf::from("one.wav")));
    assert!(selected.contains(&PathBuf::from("two.wav")));
    assert!(selected.contains(&PathBuf::from("three.wav")));
    assert!(selected.contains(&PathBuf::from("six.wav")));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}

#[test]
fn shift_arrow_grows_selection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(1);
    controller.grow_selection(1);

    let selected: Vec<_> = controller
        .ui
        .browser
        .selection
        .selected_paths
        .iter()
        .cloned()
        .collect();
    assert!(selected.contains(&PathBuf::from("two.wav")));
    assert!(selected.contains(&PathBuf::from("three.wav")));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}
