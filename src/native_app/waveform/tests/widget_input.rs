use super::super::WaveformActiveDragKind;
use super::*;

static WIDGET_INPUT_CONFIG_BASE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_widget_input_test_config_base(
    path: std::path::PathBuf,
) -> (
    std::sync::MutexGuard<'static, ()>,
    wavecrate::app_dirs::ConfigBaseGuard,
) {
    let lock = WIDGET_INPUT_CONFIG_BASE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = wavecrate::app_dirs::ConfigBaseGuard::set(path);
    (lock, guard)
}

#[test]
fn auxiliary_drag_pans_zoomed_waveform_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 12_000,
        end: 36_000,
    };
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(100.0, 40.0),
                PointerButton::Auxiliary,
                Default::default(),
            ),
        )
        .expect("middle press should arm waveform pan");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
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
fn playback_cache_backed_waveform_accepts_primary_click() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_widget_input_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("cached-widget-input.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);

    let full_waveform = WaveformState::load_path(sample_path.clone()).expect("cache sample");
    let file = full_waveform.file();
    super::super::store_cached_waveform_file_for_tests(&file);
    let cached_waveform =
        WaveformState::load_persisted_playback_cache(sample_path).expect("playback cache loads");
    assert!(cached_waveform.has_loaded_sample());
    assert!(
        cached_waveform.audio_bytes().is_empty(),
        "persisted playback cache should not need source WAV bytes on the UI path"
    );
    let mut widget = waveform_widget_for_state(&cached_waveform);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(60.0, 40.0)))
        .expect("playback-cache-backed waveform should accept primary input");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
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
fn primary_press_emits_playback_ratio_matching_hover_cursor_ratio() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    assert!(widget.accepts_pointer_move());

    let hover = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 40.0)));
    let hover_cursor_ratio = widget.hover_cursor_ratio;

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(60.0, 40.0)))
        .expect("playback interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert!(hover.is_none());
    assert_eq!(hover_cursor_ratio, Some(0.25));
    assert_eq!(widget.hover_cursor_ratio, None);
    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25
        }
    );
}

#[test]
fn pointer_move_outside_loaded_waveform_clears_hover_cursor() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    widget.hover_cursor_ratio = Some(0.25);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(240.0, 40.0)));

    assert!(output.is_none());
    assert_eq!(widget.hover_cursor_ratio, None);
    assert!(!widget.common.is_hovered());
}

#[test]
fn active_selection_pointer_move_outside_waveform_updates_to_nearest_edge() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.hover_cursor_ratio = Some(0.25);

    let left_output = widget
        .handle_input(bounds, WidgetInput::pointer_move(Point::new(0.0, 40.0)))
        .expect("active drag should continue outside the left edge");
    let left_interaction = left_output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        left_interaction,
        WaveformInteraction::UpdateSelection { visible_ratio: 0.0 }
    );
    assert_eq!(widget.hover_cursor_ratio, None);
    assert!(!widget.common.is_hovered());

    let right_output = widget
        .handle_input(bounds, WidgetInput::pointer_move(Point::new(240.0, 40.0)))
        .expect("active drag should continue outside the right edge");
    let right_interaction = right_output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        right_interaction,
        WaveformInteraction::UpdateSelection { visible_ratio: 1.0 }
    );
    assert!(!widget.common.is_hovered());
}

#[test]
fn captured_selection_drag_outside_waveform_updates_to_nearest_edge() {
    for (button, kind) in [
        (PointerButton::Primary, WaveformSelectionKind::Play),
        (PointerButton::Secondary, WaveformSelectionKind::Edit),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        let mut widget = waveform_widget_for_state(&state);
        let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
        let begin_output = widget
            .handle_input(
                bounds,
                WidgetInput::pointer_press(Point::new(160.0, 40.0), button, Default::default()),
            )
            .expect("press should begin a selection drag");
        let begin_interaction = begin_output
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");

        assert_eq!(
            begin_interaction,
            WaveformInteraction::BeginSelection {
                kind,
                visible_ratio: 0.75,
            }
        );

        state.apply_interaction(begin_interaction);
        widget.active_drag_kind = state.active_drag_kind();

        let left_output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(0.0, 40.0)))
            .expect("captured drag should continue outside the left edge");
        let left_interaction = left_output
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");

        assert_eq!(
            left_interaction,
            WaveformInteraction::UpdateSelection { visible_ratio: 0.0 }
        );

        let right_output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(240.0, 40.0)))
            .expect("captured drag should continue outside the right edge");
        let right_interaction = right_output
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");

        assert_eq!(
            right_interaction,
            WaveformInteraction::UpdateSelection { visible_ratio: 1.0 }
        );
    }
}

#[test]
fn pointer_move_updates_hover_cursor_locally_without_host_message() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 40.0)));

    assert!(output.is_none());
    assert!(widget.common.is_hovered());
    assert_eq!(widget.hover_cursor_ratio, Some(0.25));
    assert!(widget.prefers_pointer_move_paint_only());
}

#[test]
fn pointer_move_over_similar_section_uses_region_hover_instead_of_cursor() {
    let mut state = WaveformState::synthetic_for_tests();
    let similar = wavecrate::selection::SelectionRange::new(0.2, 0.6);
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![similar]);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(80.0, 40.0)));

    assert!(output.is_none());
    assert!(widget.common.is_hovered());
    assert_eq!(widget.hovered_similar_section, Some(similar));
    assert_eq!(widget.hover_cursor_ratio, None);
}

#[test]
fn pressing_similar_section_selects_it_as_flashing_edit_selection() {
    for button in [PointerButton::Primary, PointerButton::Secondary] {
        let mut state = WaveformState::synthetic_for_tests();
        let similar = wavecrate::selection::SelectionRange::new(0.2, 0.6);
        state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
        state.finish_similar_sections_scan(vec![similar]);
        let mut widget = waveform_widget_for_state(&state);
        let bounds = Rect::from_size(200.0, 80.0);

        let interaction = widget
            .handle_input(
                bounds,
                WidgetInput::pointer_press(Point::new(80.0, 40.0), button, Default::default()),
            )
            .expect("similar section press should select the region")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::SelectSimilarSection { selection: similar }
        );
        state.apply_interaction(interaction);
        assert_eq!(state.edit_selection(), Some(similar));
        assert!(state.edit_selection_flash_frames() > 0);
        assert_eq!(state.similar_section_ranges(), &[similar]);
    }
}

#[test]
fn secondary_press_emits_edit_selection_begin_ratio() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(160.0, 40.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("edit selection interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
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
fn secondary_click_with_tiny_motion_clears_edit_selection_without_micro_range() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let press = Point::new(150.0, 40.0);
    let tiny_move = Point::new(152.0, 40.0);

    let begin = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(press, PointerButton::Secondary, Default::default()),
        )
        .expect("secondary click should begin edit selection handling")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let move_output = widget.handle_input(bounds, WidgetInput::pointer_move(tiny_move));
    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(tiny_move, PointerButton::Secondary, Default::default()),
        )
        .expect("secondary release should finish edit click")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert!(move_output.is_none());
    assert_eq!(state.edit_selection(), None);
    assert_eq!(state.edit_mark_ratio(), None);
}

#[test]
fn primary_click_with_tiny_motion_clears_play_selection_without_micro_range() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let press = Point::new(150.0, 40.0);
    let tiny_move = Point::new(152.0, 40.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(press))
        .expect("primary click should begin play selection handling")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let move_output = widget.handle_input(bounds, WidgetInput::pointer_move(tiny_move));
    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(tiny_move, PointerButton::Primary, Default::default()),
        )
        .expect("primary release should finish play click")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert!(move_output.is_none());
    assert!(state.is_playing());
    assert_eq!(state.play_selection(), None);
    assert_eq!(state.play_mark_ratio(), Some(0.75));
}

#[test]
fn primary_drag_three_pixels_starts_playmark_selection_update() {
    let mut state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let press = Point::new(40.0, 40.0);
    let drag = Point::new(43.0, 40.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(press))
        .expect("primary press should begin playmark selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let update = widget
        .handle_input(bounds, WidgetInput::pointer_move(drag))
        .expect("three-pixel drag should start the playmark range")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(update);

    let selection = state.play_selection().expect("playmark selection");
    assert!((selection.start() - 0.2).abs() < f32::EPSILON);
    assert!((selection.end() - 0.215).abs() < f32::EPSILON);
}

#[test]
fn three_pixel_drag_starts_play_and_edit_selections_while_zoomed_in() {
    for (button, kind) in [
        (PointerButton::Primary, WaveformSelectionKind::Play),
        (PointerButton::Secondary, WaveformSelectionKind::Edit),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        state.viewport = super::WaveformViewport {
            start: 12_000,
            end: 12_256,
        };
        let mut widget = waveform_widget_for_state(&state);
        let bounds = Rect::from_size(200.0, 80.0);
        let press = Point::new(40.0, 40.0);
        let drag = Point::new(43.0, 40.0);

        let begin = widget
            .handle_input(
                bounds,
                WidgetInput::pointer_press(press, button, Default::default()),
            )
            .expect("press should begin selection handling")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        assert_eq!(
            begin,
            WaveformInteraction::BeginSelection {
                kind,
                visible_ratio: 0.2
            }
        );
        state.apply_interaction(begin);
        widget.active_drag_kind = state.active_drag_kind();

        let update = widget
            .handle_input(bounds, WidgetInput::pointer_move(drag))
            .expect("three-pixel drag should start range selection")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        state.apply_interaction(update);

        let selection = match kind {
            WaveformSelectionKind::Play => state.play_selection(),
            WaveformSelectionKind::Edit => state.edit_selection(),
        }
        .expect("selection should start immediately after click slop");
        assert!(
            selection.width() > 0.0,
            "{kind:?} selection should have non-zero width"
        );
    }
}

#[test]
fn empty_waveform_keeps_hover_cursor_but_emits_no_interactions() {
    let state = WaveformState::empty();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    let inside = Point::new(60.0, 40.0);

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(inside))
            .is_none()
    );
    assert!(
        widget.common.is_hovered(),
        "empty waveform should keep pointer-hover state for the visual cursor path"
    );
    for input in [
        WidgetInput::primary_press(inside),
        WidgetInput::pointer_press(inside, PointerButton::Secondary, Default::default()),
        WidgetInput::pointer_press(inside, PointerButton::Auxiliary, Default::default()),
        WidgetInput::primary_double_click(inside),
        WidgetInput::plain_wheel(inside, Vector2::new(0.0, -120.0)),
    ] {
        assert!(widget.handle_input(bounds, input).is_none());
    }
}

#[test]
fn shift_wheel_marks_zoom_out_as_silence_margin_expand() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::wheel(
                Point::new(60.0, 40.0),
                Vector2::new(0.0, 120.0),
                radiant::widgets::PointerModifiers {
                    shift: true,
                    ..Default::default()
                },
            ),
        )
        .expect("shift wheel should emit waveform wheel interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::Wheel {
            delta: Vector2::new(0.0, 120.0),
            anchor_ratio: 0.25,
            expand_silence_margin: true,
        }
    );
}

#[test]
fn primary_press_on_playmark_handle_starts_resize_instead_of_new_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(120.0, 8.0)))
        .expect("playmark resize interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
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
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(80.0, 3.0)))
        .expect("playmark move interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
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
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(118.0, 76.0)))
        .expect("selection export drag interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::DragPlaySelectionExport(radiant::widgets::DragHandleMessage::started(
            Point::new(118.0, 76.0)
        ))
    );
}

#[test]
fn secondary_press_on_playmark_body_opens_context_menu() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(80.0, 40.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("playmark context menu interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::OpenPlaySelectionContextMenu {
            position: Point::new(80.0, 40.0)
        }
    );
}

#[test]
fn secondary_press_on_edit_top_handle_starts_move() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(60.0, 3.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("edit move interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionMove {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.3
        }
    );
}

#[test]
fn primary_press_on_edit_bottom_handle_starts_resize() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(120.0, 76.0)))
        .expect("edit resize interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Edit,
            edge: WaveformSelectionEdge::End,
            visible_ratio: 0.6
        }
    );
}

#[test]
fn secondary_press_on_edit_bottom_handle_starts_resize() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(40.0, 76.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("edit resize interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Edit,
            edge: WaveformSelectionEdge::Start,
            visible_ratio: 0.2
        }
    );
}

#[test]
fn secondary_press_on_faded_edit_bottom_side_does_not_resize() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.4));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(120.0, 76.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("edit fade interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::OutEnd,
            visible_ratio: 0.6
        }
    );
}

#[test]
fn primary_press_on_edit_gain_handle_starts_gain_drag_instead_of_move() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(80.0, 5.0)))
        .expect("edit gain interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditGain { pointer_y: 5.0 }
    );
}

#[test]
fn active_edit_gain_drag_emits_vertical_updates_and_finish() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(80.0, 5.0)))
        .expect("edit gain begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let update = widget
        .handle_input(bounds, WidgetInput::pointer_move(Point::new(80.0, 45.0)))
        .expect("edit gain update")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        update,
        WaveformInteraction::UpdateEditGain { pointer_y: 45.0 }
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(80.0, 45.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("edit gain finish")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        finish,
        WaveformInteraction::FinishEditGain { pointer_y: 45.0 }
    );
}

#[test]
fn secondary_press_on_edit_gain_handle_starts_gain_drag() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(80.0, 5.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("edit gain interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditGain { pointer_y: 5.0 }
    );
}

#[test]
fn primary_press_on_edit_fade_outer_gain_handle_starts_outer_gain_drag() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.25),
    );
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 0.0)))
        .expect("edit fade outer gain interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditFadeOuterGain {
            handle: WaveformEditFadeOuterGainHandle::In,
            vertical_ratio: 0.0
        }
    );
}

#[test]
fn active_edit_fade_outer_gain_drag_emits_vertical_updates_and_finish() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_out(0.25, 0.7)
            .with_fade_out_mute(0.25),
    );
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(140.0, 0.0)))
        .expect("edit fade outer gain begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let update = widget
        .handle_input(bounds, WidgetInput::pointer_move(Point::new(140.0, 40.0)))
        .expect("edit fade outer gain update")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        update,
        WaveformInteraction::UpdateEditFadeOuterGain {
            vertical_ratio: 0.5
        }
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(140.0, 40.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("edit fade outer gain finish")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        finish,
        WaveformInteraction::FinishEditFadeOuterGain {
            vertical_ratio: 0.5
        }
    );
}

#[test]
fn primary_press_on_edit_fade_handle_starts_fade_drag_instead_of_playmark() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(40.0, 4.0)))
        .expect("fade handle interaction");
    let interaction = output
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        interaction,
        WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::InEnd,
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
