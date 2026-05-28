use super::*;

#[test]
fn auxiliary_drag_pans_zoomed_waveform_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 12_000,
        end: 36_000,
    };
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(100.0, 40.0),
                button: PointerButton::Auxiliary,
                modifiers: Default::default(),
            },
        )
        .expect("middle press should arm waveform pan");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform pan interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginPan { visible_ratio: 0.5 }
    );
    state.apply_interaction(interaction);
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.25,
    });

    assert!(
        state.viewport().start > 12_000,
        "dragging left should pan the viewport later in the sample"
    );
    assert_eq!(state.viewport().visible_items(), 24_000);
}

#[test]
fn primary_press_emits_playback_ratio_matching_hover_cursor_ratio() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(60.0, 40.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("playback interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25
        }
    );
}

#[test]
fn secondary_press_emits_edit_selection_begin_ratio() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(160.0, 40.0),
                button: PointerButton::Secondary,
                modifiers: Default::default(),
            },
        )
        .expect("edit selection interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.75
        }
    );
}

#[test]
fn empty_waveform_keeps_hover_cursor_but_emits_no_interactions() {
    let state = WaveformState::empty();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));
    let inside = Point::new(60.0, 40.0);

    assert!(
        widget
            .handle_input(bounds, WidgetInput::PointerMove { position: inside },)
            .is_none()
    );
    assert!(
        widget.common.state.hovered,
        "empty waveform should keep pointer-hover state for the visual cursor path"
    );
    for input in [
        WidgetInput::PointerPress {
            position: inside,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
        WidgetInput::PointerPress {
            position: inside,
            button: PointerButton::Secondary,
            modifiers: Default::default(),
        },
        WidgetInput::PointerPress {
            position: inside,
            button: PointerButton::Auxiliary,
            modifiers: Default::default(),
        },
        WidgetInput::PointerDoubleClick {
            position: inside,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
        WidgetInput::Wheel {
            position: inside,
            delta: Vector2::new(0.0, -120.0),
            modifiers: Default::default(),
        },
    ] {
        assert!(widget.handle_input(bounds, input).is_none());
    }
}

#[test]
fn primary_press_on_playmark_handle_starts_resize_instead_of_new_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(120.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("playmark resize interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::End,
            visible_ratio: 0.6
        }
    );
}

#[test]
fn primary_press_on_playmark_top_handle_starts_move() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(80.0, 3.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("playmark move interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionMove {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.4
        }
    );
}

#[test]
fn primary_press_on_play_selection_export_handle_starts_export_drag() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(118.0, 76.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("selection export drag interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::DragPlaySelectionExport(
            radiant::widgets::DragHandleMessage::Started {
                position: Point::new(118.0, 76.0)
            }
        )
    );
}

#[test]
fn secondary_press_on_edit_top_handle_starts_move() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(80.0, 3.0),
                button: PointerButton::Secondary,
                modifiers: Default::default(),
            },
        )
        .expect("edit move interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionMove {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.4
        }
    );
}

#[test]
fn primary_press_on_edit_fade_handle_starts_fade_drag_instead_of_playmark() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(40.0, 4.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("fade handle interaction");
    let interaction = output
        .typed_ref::<WaveformInteraction>()
        .copied()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInEnd,
            visible_ratio: 0.2
        }
    );
}

#[test]
fn primary_click_without_drag_still_starts_playback_from_click() {
    let mut state = WaveformState::synthetic_for_tests();

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
