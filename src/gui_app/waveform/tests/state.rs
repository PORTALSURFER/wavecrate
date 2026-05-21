use super::*;

#[test]
fn playback_state_starts_at_head_and_clears_on_stop() {
    let mut state = WaveformState::synthetic_for_tests();

    assert!(!state.is_playing());
    assert_eq!(state.playhead_ratio(), None);
    assert_eq!(state.play_mark_ratio(), None);

    state.start_playback(0.0);
    assert!(state.is_playing());
    assert_eq!(state.playhead_ratio(), Some(0.0));
    assert_eq!(state.play_mark_ratio(), Some(0.0));

    state.set_playhead_ratio(0.375);
    assert_eq!(state.playhead_ratio(), Some(0.375));
    assert_eq!(state.play_mark_ratio(), Some(0.0));

    state.stop_playback();
    assert!(!state.is_playing());
    assert_eq!(state.playhead_ratio(), None);
    assert_eq!(state.play_mark_ratio(), Some(0.0));
}

#[test]
fn visible_ratio_maps_to_absolute_audio_position_inside_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 12_000,
        end: 36_000,
    };

    let ratio = state.absolute_ratio_from_visible(0.5);

    assert!((ratio - 0.5).abs() < 0.0001);
}

#[test]
fn dragging_primary_creates_playmark_selection_without_starting_playback() {
    let mut state = WaveformState::synthetic_for_tests();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });

    let selection = state.play_selection().expect("playmark selection");
    assert!(!state.is_playing());
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert_eq!(state.play_mark_ratio(), Some(0.2));
}

#[test]
fn playmark_range_edges_are_resizable() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelectionResize {
        kind: WaveformSelectionKind::Play,
        edge: WaveformSelectionEdge::End,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.75,
    });

    let selection = state.play_selection().expect("playmark selection");
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.75).abs() < 0.001);
    assert_eq!(state.play_mark_ratio(), Some(selection.start()));
    assert!(!state.is_playing());
}

#[test]
fn playmark_top_handle_moves_range_without_resizing() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelectionMove {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.4,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.55,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.55,
    });

    let selection = state.play_selection().expect("moved playmark selection");
    assert!((selection.start() - 0.35).abs() < 0.001);
    assert!((selection.end() - 0.75).abs() < 0.001);
    assert!((selection.width() - 0.4).abs() < 0.001);
    assert_eq!(state.play_mark_ratio(), Some(selection.start()));
    assert!(!state.is_playing());
}

#[test]
fn edit_top_handle_moves_range_and_preserves_edit_effects() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_out(0.25, 0.7),
    );
    state.edit_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelectionMove {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.4,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.1 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

    let selection = state.edit_selection().expect("moved edit selection");
    assert!((selection.start() - 0.0).abs() < 0.001);
    assert!((selection.end() - 0.4).abs() < 0.001);
    assert_eq!(state.edit_mark_ratio(), Some(selection.start()));
    assert_eq!(selection.fade_in().map(|fade| fade.length), Some(0.25));
    assert_eq!(selection.fade_out().map(|fade| fade.length), Some(0.25));
}

#[test]
fn dragging_secondary_creates_edit_selection() {
    let mut state = WaveformState::synthetic_for_tests();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.7,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.25,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.25,
    });

    let selection = state.edit_selection().expect("edit selection");
    assert!((selection.start() - 0.25).abs() < 0.001);
    assert!((selection.end() - 0.7).abs() < 0.001);
    assert_eq!(state.edit_mark_ratio(), Some(0.7));
}
