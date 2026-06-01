use super::*;

#[test]
fn edit_fade_out_bottom_handle_keeps_collapsed_silence_after_release() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(1.0),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 1.0 });
    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 1.0,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_out = selection.fade_out().expect("fade-out should remain");
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.end() - 0.7).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.000_001);
}

#[test]
fn double_click_outer_fade_handles_collapses_silence_without_clearing_fade() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.5)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(0.75),
    );

    state.apply_interaction(WaveformInteraction::ClearEditFadeSilence {
        handle: WaveformEditFadeHandle::InOuterStart,
    });
    state.apply_interaction(WaveformInteraction::ClearEditFadeSilence {
        handle: WaveformEditFadeHandle::OutOuterEnd,
    });

    let selection = state.edit_selection().expect("edit selection");
    let fade_in = selection.fade_in().expect("fade-in should remain");
    let fade_out = selection.fade_out().expect("fade-out should remain");
    assert!((fade_in.length - 0.25).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!(fade_in.mute.abs() < 0.001);
    assert!((fade_out.length - 0.25).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
    assert!(fade_out.mute.abs() < 0.001);
}

#[test]
fn double_click_on_outer_fade_handle_emits_silence_clear_interaction() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(0.25),
    );
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::primary_double_click(Point::new(140.0, 40.0)),
        )
        .expect("outer fade double-click interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::ClearEditFadeSilence {
            handle: WaveformEditFadeHandle::OutOuterEnd
        }
    );
}

#[test]
fn edit_fade_out_top_handle_preserves_silence_after_bottom_handle_collapse() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(1.0),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.5 });
    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutStart,
        visible_ratio: 0.5,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.45,
    });

    let selection = state.edit_selection().expect("edit selection");
    let fade_out = selection
        .fade_out()
        .expect("fade-out silence handle should remain");
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.end() - 0.5).abs() < 0.001);
    assert!((fade_out_start - 0.45).abs() < 0.001);
    assert!((fade_out_outer_end - 1.0).abs() < 0.000_001);
}

#[test]
fn edit_fade_out_bottom_handle_keeps_left_crossfade_pinned_to_sample_edge() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.5)
            .with_fade_out(0.25, 0.7),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_in = selection.fade_in().expect("fade-in should remain");
    let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;

    assert!(fade_in_outer_start.abs() < 0.000_001);
}

#[test]
fn edit_fade_out_bottom_handle_keeps_left_crossfade_pinned_across_wiggles() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.5)
            .with_fade_out(0.25, 0.7),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    for visible_ratio in [0.7, 0.69, 0.71, 0.705, 0.7] {
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio });
        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
        assert!(
            fade_in_outer_start.abs() < 0.000_001,
            "left silence handle drifted to {fade_in_outer_start}"
        );
    }
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });
}

#[test]
fn edit_fade_in_bottom_handle_keeps_opposite_fade_boundary_stable() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_out(0.25, 0.7),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InStart,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_in = selection.fade_in().expect("fade-in should remain");
    let fade_out = selection.fade_out().expect("fade-out should remain");
    let fade_in_end = selection.start() + selection.width() * fade_in.length;
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

#[test]
fn edit_fade_in_bottom_handle_keeps_crossfade_handles_stable() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.25)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(0.25),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InStart,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_in = selection.fade_in().expect("fade-in should remain");
    let fade_out = selection.fade_out().expect("fade-out should remain");
    let fade_in_end = selection.start() + selection.width() * fade_in.length;
    let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.start() - 0.1).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_in_outer_start - 0.1).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}
