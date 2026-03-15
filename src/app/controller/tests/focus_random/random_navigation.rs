use super::*;

#[test]
fn random_sample_selection_uses_seeded_rng() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let mut rng = StdRng::seed_from_u64(99);
    let expected = visible_indices(&controller)
        .into_iter()
        .enumerate()
        .choose(&mut rng)
        .map(|(row, _)| row);

    controller.play_random_visible_sample_with_seed(99);

    assert_eq!(controller.ui.browser.selection.selected_visible, expected);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        expected
    );
    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert!(controller.ui.browser.selection.autoscroll);
}

#[test]
fn random_sample_hotkey_is_registered() {
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "play-random-sample")
        .expect("play-random-sample hotkey");
    assert_eq!(action.label, "Play random sample");
    assert!(action.is_global());
    assert_eq!(action.gesture.first.key, KeyCode::R);
    assert!(action.gesture.first.shift);
    assert!(!action.gesture.first.command);
    assert!(action.gesture.chord.is_none());
}

#[test]
fn random_history_hotkey_is_registered() {
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "play-previous-random-sample")
        .expect("play-previous-random-sample hotkey");
    assert_eq!(action.label, "Play previous random sample");
    assert!(action.is_global());
    assert_eq!(action.gesture.first.key, KeyCode::R);
    assert!(action.gesture.first.shift);
    assert!(action.gesture.first.command);
    assert!(action.gesture.chord.is_none());
}

#[test]
fn random_navigation_toggle_hotkey_is_registered() {
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "toggle-random-navigation-mode")
        .expect("toggle-random-navigation-mode hotkey");
    assert_eq!(action.label, "Toggle random navigation mode");
    assert!(action.is_global());
    assert_eq!(action.gesture.first.key, KeyCode::R);
    assert!(action.gesture.first.alt);
    assert!(!action.gesture.first.shift);
    assert!(!action.gesture.first.command);
    assert!(action.gesture.chord.is_none());
}

#[test]
fn random_history_steps_backward() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let mut rng = StdRng::seed_from_u64(5);
    let first_expected = visible_indices(&controller)
        .into_iter()
        .enumerate()
        .choose(&mut rng)
        .map(|(row, _)| row);
    controller.play_random_visible_sample_with_seed(5);

    let mut rng = StdRng::seed_from_u64(9);
    let second_expected = visible_indices(&controller)
        .into_iter()
        .enumerate()
        .choose(&mut rng)
        .map(|(row, _)| row);
    controller.play_random_visible_sample_with_seed(9);

    assert_eq!(
        controller.ui.browser.selection.selected_visible,
        second_expected
    );
    assert_eq!(controller.history.random_history.cursor, Some(1));

    controller.play_previous_random_sample();

    assert_eq!(controller.history.random_history.cursor, Some(0));
    assert_eq!(
        controller.ui.browser.selection.selected_visible,
        first_expected
    );
}

#[test]
fn random_history_trims_to_limit() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let total = RANDOM_HISTORY_LIMIT + 5;
    controller.set_wav_entries_for_tests(
        (0..total)
            .map(|i| sample_entry(&format!("{i}.wav"), crate::sample_sources::Rating::NEUTRAL))
            .collect(),
    );
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    for seed in 0..total as u64 {
        controller.play_random_visible_sample_with_seed(seed);
    }

    assert_eq!(
        controller.history.random_history.entries.len(),
        RANDOM_HISTORY_LIMIT
    );
    assert_eq!(
        controller.history.random_history.cursor,
        Some(
            controller
                .history
                .random_history
                .entries
                .len()
                .saturating_sub(1)
        )
    );
}

#[test]
fn random_sample_handles_empty_lists() {
    let (mut controller, _source) = dummy_controller();

    controller.play_random_visible_sample_with_seed(7);

    assert_eq!(
        controller.ui.status.text,
        "No samples available to randomize"
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, None);
}

#[test]
fn random_navigation_mode_toggles_state_and_status() {
    let (mut controller, _source) = dummy_controller();

    assert!(!controller.random_navigation_mode_enabled());

    controller.toggle_random_navigation_mode();

    assert!(controller.random_navigation_mode_enabled());
    assert_eq!(
        controller.ui.status.text,
        "Random navigation on: Up/Down jump to random samples"
    );

    controller.toggle_random_navigation_mode();

    assert!(!controller.random_navigation_mode_enabled());
    assert_eq!(controller.ui.status.text, "Random navigation off");
}

#[test]
fn toggling_random_navigation_marks_current_focus_as_visited() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.toggle_random_navigation_mode();

    assert!(
        controller
            .history
            .random_history
            .has_played(&source.id, Path::new("one.wav"))
    );
    assert!(
        !controller
            .history
            .random_history
            .has_played(&source.id, Path::new("two.wav"))
    );
}

#[test]
fn random_sample_navigation_avoids_repeats() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let mut played = std::collections::HashSet::new();

    for _ in 0..3 {
        controller.play_random_visible_sample();
        let selected = controller
            .ui
            .browser
            .selection
            .selected_visible
            .expect("sample selected");
        let path = controller
            .visible_browser_index(selected)
            .and_then(|idx| controller.wav_entry(idx))
            .map(|e| e.relative_path.clone())
            .expect("path");
        assert!(played.insert(path), "Sample should not be repeated");
    }

    assert_eq!(played.len(), 3, "All samples should have been played");

    controller.play_random_visible_sample();
    let selected = controller
        .ui
        .browser
        .selection
        .selected_visible
        .expect("sample selected");
    let path = controller
        .visible_browser_index(selected)
        .and_then(|idx| controller.wav_entry(idx))
        .map(|e| e.relative_path.clone())
        .expect("path");
    assert!(
        played.contains(&path),
        "Should repeat after all were played"
    );
}
