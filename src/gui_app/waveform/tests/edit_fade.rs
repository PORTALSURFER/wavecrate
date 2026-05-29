use super::*;

#[test]
fn edit_fade_top_handle_drag_sets_fade_in_length() {
    let mut state = WaveformState::synthetic_for_tests();
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InEnd,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.3 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.3 });

    let selection = state.edit_selection().expect("edit selection");
    let fade = selection.fade_in().expect("fade-in after handle drag");
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert!((fade.length - 0.25).abs() < 0.001);
    assert!((fade.curve - 0.5).abs() < 0.001);
}

#[test]
fn edit_fade_top_handles_push_and_restore_opposite_fade() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_out(0.25, 0.7),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InEnd,
        visible_ratio: 0.3,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
    let pushed = state.edit_selection().expect("pushed edit selection");
    let pushed_fade_in = pushed.fade_in().expect("fade-in after push");
    assert!(pushed.fade_out().is_none());
    assert!((pushed.start() + pushed.width() * pushed_fade_in.length - 0.6).abs() < 0.001);

    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.3 });
    let restored = state.edit_selection().expect("restored edit selection");
    let restored_fade_in = restored.fade_in().expect("restored fade-in");
    let restored_fade_out = restored.fade_out().expect("restored fade-out");
    let fade_in_end = restored.start() + restored.width() * restored_fade_in.length;
    let fade_out_start = restored.end() - restored.width() * restored_fade_out.length;
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((restored_fade_in.curve - 0.2).abs() < 0.001);
    assert!((restored_fade_out.curve - 0.7).abs() < 0.001);
}

#[test]
fn edit_fade_outer_handles_set_crossfade_lengths_without_resizing_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InOuterStart,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

    let selection = state.edit_selection().expect("edit selection");
    let fade = selection.fade_in().expect("fade-in after outer drag");
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert!((fade.length - 0.25).abs() < 0.001);
    assert!((fade.mute - 0.25).abs() < 0.001);

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InOuterStart,
        visible_ratio: 0.1,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.2 });

    let selection = state.edit_selection().expect("edit selection");
    let fade = selection.fade_in().expect("fade-in should remain");
    assert!((fade.length - 0.25).abs() < 0.001);
    assert!(fade.mute.abs() < 0.001);

    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.7));
    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutOuterEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

    let selection = state.edit_selection().expect("edit selection");
    let fade = selection.fade_out().expect("fade-out after outer drag");
    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert!((fade.length - 0.25).abs() < 0.001);
    assert!((fade.mute - 0.25).abs() < 0.001);
}

#[test]
fn primary_press_on_outer_fade_handle_uses_distinct_handle() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(40.0, 40.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("outer fade handle interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::InOuterStart,
            visible_ratio: 0.2
        }
    );
}

#[test]
fn edit_fade_bottom_handle_resizes_selection_and_keeps_fade_boundary() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::InStart,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

    let selection = state.edit_selection().expect("edit selection");
    let fade = selection.fade_in().expect("fade-in after resize");
    assert!((selection.start() - 0.1).abs() < 0.001);
    assert!((selection.end() - 0.6).abs() < 0.001);
    assert!((selection.start() + selection.width() * fade.length - 0.3).abs() < 0.001);
    assert!((fade.curve - 0.2).abs() < 0.001);
}

#[test]
fn edit_fade_out_bottom_handle_keeps_opposite_fade_boundary_stable() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_out(0.25, 0.7),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.8 });

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
fn edit_fade_out_bottom_handle_keeps_crossfade_handles_stable() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.25)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(0.25),
    );

    state.apply_interaction(WaveformInteraction::BeginEditFade {
        handle: WaveformEditFadeHandle::OutEnd,
        visible_ratio: 0.6,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_in = selection.fade_in().expect("fade-in should remain");
    let fade_out = selection.fade_out().expect("fade-out should remain");
    let fade_in_end = selection.start() + selection.width() * fade_in.length;
    let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.7).abs() < 0.001);
    assert!((fade_in_end - 0.3).abs() < 0.001);
    assert!((fade_in_outer_start - 0.1).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 0.7).abs() < 0.001);
    assert!((fade_in.curve - 0.2).abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

#[test]
fn edit_fade_out_bottom_handle_preserves_crossfade_when_fade_collapses() {
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

    let selection = state.edit_selection().expect("edit selection");
    let fade_out = selection
        .fade_out()
        .expect("fade-out silence handle should remain");
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.start() - 0.2).abs() < 0.001);
    assert!((selection.end() - 0.5).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 1.0).abs() < 0.001);
    assert!(fade_out.length.abs() < 0.001);
    assert!((fade_out.curve - 0.7).abs() < 0.001);
}

#[test]
fn edit_fade_out_bottom_handle_does_not_pick_up_silence_during_same_drag() {
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
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 1.0 });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.7 });

    let selection = state.edit_selection().expect("edit selection");
    let fade_out = selection
        .fade_out()
        .expect("fade-out silence handle should remain");
    let fade_out_start = selection.end() - selection.width() * fade_out.length;
    let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

    assert!((selection.end() - 0.7).abs() < 0.001);
    assert!((fade_out_start - 0.5).abs() < 0.001);
    assert!((fade_out_outer_end - 1.0).abs() < 0.000_001);
}
