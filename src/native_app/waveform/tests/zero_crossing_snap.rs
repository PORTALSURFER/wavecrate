use super::*;

#[test]
fn zero_crossing_snap_drags_playmark_and_editmark_edges_to_nearby_crossings() {
    let mut state = zero_crossing_snap_state();
    state.toggle_zero_crossing_snap();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.18,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.78,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.78,
    });

    let play_selection = state.play_selection().expect("snapped playmark selection");
    assert!((play_selection.start() - 0.2).abs() < 0.001);
    assert!((play_selection.end() - 0.8).abs() < 0.001);
    assert_eq!(state.play_mark_ratio(), Some(0.2));

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.82,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.22,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.22,
    });

    let edit_selection = state.edit_selection().expect("snapped editmark selection");
    assert!((edit_selection.start() - 0.2).abs() < 0.001);
    assert!((edit_selection.end() - 0.8).abs() < 0.001);
    assert_eq!(state.edit_mark_ratio(), Some(0.8));
}

#[test]
fn zero_crossing_snap_disabled_keeps_authored_selection_edges() {
    let mut state = zero_crossing_snap_state();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.18,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.78,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.78,
    });

    let selection = state
        .play_selection()
        .expect("unsnapped playmark selection");
    assert!((selection.start() - 0.18).abs() < 0.001);
    assert!((selection.end() - 0.78).abs() < 0.001);
}

#[test]
fn zero_crossing_snap_preserves_edit_effects_while_resizing() {
    let mut state = zero_crossing_snap_state();
    state.toggle_zero_crossing_snap();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.18, 0.78)
            .with_gain(0.5)
            .with_fade_in(0.25, 0.2),
    );
    state.edit_mark_ratio = Some(0.18);

    state.apply_interaction(WaveformInteraction::BeginSelectionResize {
        kind: WaveformSelectionKind::Edit,
        edge: WaveformSelectionEdge::Start,
        visible_ratio: 0.18,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.22,
    });

    let selection = state.edit_selection().expect("resized edit selection");
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.8).abs() < 0.001);
    assert!((selection.gain() - 0.5).abs() < 0.001);
    assert_eq!(selection.fade_in().map(|fade| fade.length), Some(0.25));
}

fn zero_crossing_snap_state() -> WaveformState {
    let samples = zero_crossing_snap_samples();
    let mut file = waveform_file_from_mono_samples(
        std::path::PathBuf::from("zero-crossing-snap.wav"),
        std::sync::Arc::from([0_u8]),
        1_000,
        1,
        samples.clone(),
    );
    file.playback_samples = Some(std::sync::Arc::from(samples));
    WaveformState::from_cached_file(std::sync::Arc::new(file))
}

fn zero_crossing_snap_samples() -> Vec<f32> {
    (0..100)
        .map(|frame| if (20..80).contains(&frame) { -0.5 } else { 0.5 })
        .collect()
}
