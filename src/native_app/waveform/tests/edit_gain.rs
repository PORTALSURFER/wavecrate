use super::*;

#[test]
fn edit_gain_drag_adjusts_selection_gain_without_moving_range() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

    state.apply_interaction(WaveformInteraction::BeginEditGain { pointer_y: 20.0 });
    state.apply_interaction(WaveformInteraction::UpdateEditGain { pointer_y: -20.0 });

    let boosted = state.edit_selection().expect("boosted edit selection");
    assert!((boosted.start() - 0.2).abs() < 0.001);
    assert!((boosted.end() - 0.6).abs() < 0.001);
    assert!((boosted.gain() - 2.0).abs() < 0.001);
    assert!(boosted.fade_in().is_some());
    assert_eq!(
        state.active_drag_kind(),
        Some(WaveformActiveDragKind::EditGain)
    );

    state.apply_interaction(WaveformInteraction::FinishEditGain { pointer_y: 140.0 });

    let attenuated = state.edit_selection().expect("attenuated edit selection");
    assert!((attenuated.start() - 0.2).abs() < 0.001);
    assert!((attenuated.end() - 0.6).abs() < 0.001);
    assert!((attenuated.gain() - 0.0).abs() < 0.001);
    assert!(attenuated.fade_in().is_some());
    assert_eq!(state.active_drag_kind(), None);
}

#[test]
fn edit_gain_drag_clamps_to_selection_gain_bounds() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));

    state.apply_interaction(WaveformInteraction::BeginEditGain { pointer_y: 20.0 });
    state.apply_interaction(WaveformInteraction::UpdateEditGain { pointer_y: -220.0 });

    let boosted = state.edit_selection().expect("boosted edit selection");
    assert!((boosted.gain() - 4.0).abs() < 0.001);

    state.apply_interaction(WaveformInteraction::FinishEditGain { pointer_y: 620.0 });

    let attenuated = state.edit_selection().expect("attenuated edit selection");
    assert!((attenuated.gain() - 0.0).abs() < 0.001);
}
