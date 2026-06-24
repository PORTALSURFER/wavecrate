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
fn zoom_to_play_selection_fits_active_playmark_range() {
    let mut state = WaveformState::synthetic_for_tests();
    state.set_play_selection_range(0.25, 0.50);

    state.apply_interaction(WaveformInteraction::ZoomToPlaySelection);

    assert_eq!(
        state.viewport(),
        super::WaveformViewport {
            start: 12_000,
            end: 24_000,
        }
    );
}

#[test]
fn restoring_play_selection_range_keeps_visible_region_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    let original_viewport = state.viewport();

    state.restore_play_selection_range_in_focus(0.75, 0.90);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.75, 0.90))
    );
    assert_eq!(state.play_mark_ratio(), Some(0.75));
    assert_eq!(state.viewport(), original_viewport);
    assert!(state.visible_ratio_for_absolute(0.75).is_some());
    assert!(state.visible_ratio_for_absolute(0.90).is_some());
}

#[test]
fn restoring_play_selection_range_pans_current_zoom_to_unclipped_region() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 0,
        end: 12_000,
    };

    state.restore_play_selection_range_in_focus(0.50, 0.60);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.50, 0.60))
    );
    assert_eq!(state.play_mark_ratio(), Some(0.50));
    assert_eq!(
        state.viewport(),
        super::WaveformViewport {
            start: 20_400,
            end: 32_400,
        }
    );
    assert!(state.visible_ratio_for_absolute(0.50).is_some());
    assert!(state.visible_ratio_for_absolute(0.60).is_some());
}

#[test]
fn restoring_play_selection_range_zooms_only_when_region_cannot_fit_current_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 0,
        end: 4_000,
    };

    state.restore_play_selection_range_in_focus(0.25, 0.50);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.50))
    );
    assert_eq!(state.play_mark_ratio(), Some(0.25));
    assert_eq!(
        state.viewport(),
        super::WaveformViewport {
            start: 12_000,
            end: 24_000,
        }
    );
    assert!(state.visible_ratio_for_absolute(0.25).is_some());
    assert!(state.visible_ratio_for_absolute(0.50).is_some());
}

#[test]
fn zoom_full_restores_complete_waveform_view() {
    let mut state = WaveformState::synthetic_for_tests();
    state.set_play_selection_range(0.25, 0.50);
    state.apply_interaction(WaveformInteraction::ZoomToPlaySelection);

    state.apply_interaction(WaveformInteraction::ZoomFull);

    assert_eq!(
        state.viewport(),
        super::WaveformViewport::full(state.frames())
    );
}

#[test]
fn zoom_to_tiny_play_selection_expands_to_minimum_visible_span() {
    let mut state = WaveformState::synthetic_for_tests();
    state.set_play_selection_range(0.5, 0.5001);

    state.apply_interaction(WaveformInteraction::ZoomToPlaySelection);

    assert_eq!(
        state.viewport().end - state.viewport().start,
        MIN_VISIBLE_FRAMES
    );
    assert!(state.visible_ratio_for_absolute(0.5).is_some());
}

#[test]
fn changing_playmark_selection_clears_similar_section_marks() {
    let mut state = WaveformState::synthetic_for_tests();
    state.set_play_selection_range(0.1, 0.2);
    state.start_similar_sections(state.play_selection().expect("playmark selection"));
    state.finish_similar_sections_scan(vec![wavecrate::selection::SelectionRange::new(0.5, 0.6)]);

    state.set_play_selection_range(0.2, 0.4);

    assert!(!state.similar_sections_enabled());
    assert!(state.similar_section_ranges().is_empty());
}

#[test]
fn primary_click_without_drag_clears_play_selection_and_marks_playback_start() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.45,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.45,
    });

    assert!(state.is_playing());
    assert_eq!(state.playhead_ratio(), Some(0.45));
    assert_eq!(state.play_mark_ratio(), Some(0.45));
    assert_eq!(state.play_selection(), None);
}

#[test]
fn completed_playmark_selections_are_recorded_for_random_audition() {
    let mut state = WaveformState::synthetic_for_tests();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.4 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.4 });

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.5,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.7 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

    assert_eq!(state.marked_play_ranges().len(), 1);
    assert!((state.marked_play_ranges()[0].start() - 0.2).abs() < 0.001);
    assert!((state.marked_play_ranges()[0].end() - 0.4).abs() < 0.001);
}

#[test]
fn random_marked_play_range_maps_unit_to_marked_range() {
    let ranges = [
        wavecrate::selection::SelectionRange::new(0.1, 0.2),
        wavecrate::selection::SelectionRange::new(0.3, 0.4),
        wavecrate::selection::SelectionRange::new(0.5, 0.6),
    ];

    assert_eq!(
        super::super::random_marked_play_range_for_unit(&ranges, 0.0),
        Some(ranges[0])
    );
    assert_eq!(
        super::super::random_marked_play_range_for_unit(&ranges, 0.5),
        Some(ranges[1])
    );
    assert_eq!(
        super::super::random_marked_play_range_for_unit(&ranges, 1.0),
        Some(ranges[2])
    );
}

#[test]
fn empty_waveform_ignores_selection_and_pan_interactions() {
    let mut state = WaveformState::empty();

    for interaction in [
        WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.2,
        },
        WaveformInteraction::UpdateSelection { visible_ratio: 0.6 },
        WaveformInteraction::FinishSelection { visible_ratio: 0.6 },
        WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.4,
        },
        WaveformInteraction::BeginPan { visible_ratio: 0.5 },
    ] {
        state.apply_interaction(interaction);
    }

    assert_eq!(state.play_mark_ratio(), None);
    assert_eq!(state.edit_mark_ratio(), None);
    assert_eq!(state.play_selection(), None);
    assert_eq!(state.edit_selection(), None);
    assert_eq!(state.active_drag_kind(), None);
    assert!(!state.is_playing());
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
fn edit_bottom_handle_resize_preserves_gain_and_existing_fade_shape() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_gain(0.5)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_outer_gain(0.4),
    );
    state.edit_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelectionResize {
        kind: WaveformSelectionKind::Edit,
        edge: WaveformSelectionEdge::End,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.8 });

    let selection = state.edit_selection().expect("resized edit selection");
    let fade_in = selection.fade_in().expect("fade-in should be preserved");
    let fade_in_end = selection.start() + selection.width() * fade_in.length;
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.8).abs() < 0.001);
    assert!((selection.gain() - 0.5).abs() < 0.001);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_in.outer_gain - 0.4).abs() < 0.001);
    assert_eq!(state.edit_mark_ratio(), Some(selection.start()));
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

#[test]
fn secondary_click_without_drag_clears_editmark_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.45,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.45,
    });

    assert_eq!(state.edit_mark_ratio(), None);
    assert_eq!(state.edit_selection(), None);
}
