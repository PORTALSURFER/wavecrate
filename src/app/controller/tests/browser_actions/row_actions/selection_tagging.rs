use super::*;

#[test]
fn hotkey_tagging_applies_to_all_selected_rows() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.tag_selected_left();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::TRASH_3);
    assert_eq!(controller.wav_entry(1).unwrap().tag, Rating::TRASH_3);
}

#[test]
fn x_key_toggle_respects_focus() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_focused_selection();
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.toggle_focused_selection();
    assert!(
        controller
            .ui
            .browser
            .selection
            .selected_paths
            .iter()
            .any(|path| path == &PathBuf::from("one.wav"))
    );
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}

#[test]
fn action_rows_include_selection_and_primary() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.set_browser_selected_indices(vec![0, 2]);

    let rows = controller.action_rows_from_primary(1);

    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn tag_actions_apply_to_all_selected_rows() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    controller
        .tag_browser_samples(&rows, Rating::KEEP_1, 0)
        .unwrap();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::KEEP_1);
    assert_eq!(controller.wav_entry(1).unwrap().tag, Rating::KEEP_1);
}

#[test]
fn selection_persists_when_nudging_focus() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(1);
    controller.nudge_selection(1);

    let selected = &controller.ui.browser.selection.selected_paths;
    assert!(selected.contains(&PathBuf::from("one.wav")));
    assert!(selected.contains(&PathBuf::from("two.wav")));
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}

#[test]
fn focused_row_actions_work_without_explicit_selection() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.settings.controls.advance_after_rating = false;
    controller.nudge_selection(0);
    assert!(controller.ui.browser.selection.selected_paths.is_empty());

    controller.tag_selected_left();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::TRASH_3);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
}

#[test]
fn nudge_selection_uses_random_mode_pool_without_repeating_current_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.focus_browser_row_only(0);
    controller.toggle_random_navigation_mode();
    controller
        .history
        .random_history
        .mark_played(&source.id, Path::new("two.wav"));

    controller.nudge_selection(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}
