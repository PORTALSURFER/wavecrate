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

    let full_waveform = WaveformState::load_path_for_looped_foreground_audition(
        sample_path.clone(),
        |_| {},
        || false,
        |_| {},
    )
    .expect("cache sample");
    super::super::flush_background_waveform_cache_stores_for_shutdown();
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

    assert_eq!(
        hover.and_then(|output| output.typed_copied::<WaveformInteraction>()),
        Some(WaveformInteraction::RememberPointerLocation {
            position: Point::new(60.0, 40.0)
        })
    );
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
fn command_option_primary_drag_starts_sample_slide_instead_of_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 12_000,
        end: 36_000,
    };
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let modifiers = PointerModifiers {
        command: true,
        alt: true,
        ..Default::default()
    };

    let begin = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(Point::new(50.0, 40.0), PointerButton::Primary, modifiers),
        )
        .expect("command-option press should begin sample slide")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        begin,
        WaveformInteraction::BeginSampleSlide {
            visible_ratio: 0.25
        }
    );
    state.apply_interaction(begin);

    let mut current = waveform_widget_for_state(&state);
    Widget::synchronize_from_previous(&mut current, &widget);
    let update = current
        .handle_input(bounds, WidgetInput::pointer_move(Point::new(100.0, 40.0)))
        .expect("sample slide drag should update")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        update,
        WaveformInteraction::UpdateSampleSlide { visible_ratio: 0.5 }
    );
    assert_eq!(
        current.sample_slide_frame_offset,
        Some(6_000),
        "slide preview should use current viewport frames"
    );

    let finish = current
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(100.0, 40.0),
                PointerButton::Primary,
                modifiers,
            ),
        )
        .expect("sample slide release should finish")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        finish,
        WaveformInteraction::FinishSampleSlide { visible_ratio: 0.5 }
    );
}

#[test]
fn ordinary_primary_drag_still_starts_play_selection() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(50.0, 40.0)))
        .expect("plain primary press should begin play selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");

    assert_eq!(
        output,
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
fn active_selection_pointer_move_outside_waveform_stays_paint_only() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.hover_cursor_ratio = Some(0.25);

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(0.0, 40.0)))
            .is_none(),
        "active selection drag motion should repaint locally without reducer output"
    );
    assert_eq!(widget.hover_cursor_ratio, None);
    assert!(!widget.common.is_hovered());

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(240.0, 40.0)))
            .is_none(),
        "active selection drag motion should remain paint-only at the right edge"
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

        assert!(
            widget
                .handle_input(bounds, WidgetInput::pointer_move(Point::new(0.0, 40.0)))
                .is_none(),
            "captured creation drag should update its local preview without reducer output"
        );
        let left_selection = widget
            .live_selection_preview
            .expect("left-edge live preview")
            .selection;
        assert!((left_selection.start() - 0.0).abs() < f32::EPSILON);
        assert!((left_selection.end() - 0.75).abs() < f32::EPSILON);

        assert!(
            widget
                .handle_input(bounds, WidgetInput::pointer_move(Point::new(240.0, 40.0)))
                .is_none(),
            "captured creation drag should keep using local preview at the right edge"
        );
        let right_selection = widget
            .live_selection_preview
            .expect("right-edge live preview")
            .selection;
        assert!((right_selection.start() - 0.75).abs() < f32::EPSILON);
        assert!((right_selection.end() - 1.0).abs() < f32::EPSILON);
    }
}

#[test]
fn pointer_move_updates_hover_cursor_and_remembers_context_menu_position() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 40.0)));

    assert_eq!(
        output.and_then(|output| output.typed_copied::<WaveformInteraction>()),
        Some(WaveformInteraction::RememberPointerLocation {
            position: Point::new(60.0, 40.0)
        })
    );
    assert!(widget.common.is_hovered());
    assert_eq!(widget.hover_cursor_ratio, Some(0.25));
    assert!(widget.prefers_pointer_move_paint_only());
}

#[test]
fn hover_cursor_survives_widget_synchronization() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_xy_size(10.0, 20.0, 200.0, 80.0);
    widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 40.0)));
    assert_eq!(widget.hover_cursor_ratio, Some(0.25));

    let mut rebuilt = waveform_widget_for_state(&state);
    rebuilt.synchronize_from_previous(&widget);

    assert!(rebuilt.common.is_hovered());
    assert_eq!(
        rebuilt.hover_cursor_ratio,
        Some(0.25),
        "paint-only hover cursor state should survive frame-clock widget synchronization"
    );
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

    assert_eq!(
        output.and_then(|output| output.typed_copied::<WaveformInteraction>()),
        Some(WaveformInteraction::RememberPointerLocation {
            position: Point::new(80.0, 40.0)
        })
    );
    assert!(widget.common.is_hovered());
    assert_eq!(widget.hovered_similar_section, Some(similar));
    assert_eq!(widget.hover_cursor_ratio, None);
}

#[test]
fn pointer_move_hits_clipped_similar_section_in_zoomed_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    let similar = wavecrate::selection::SelectionRange::new(0.1, 0.3);
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![similar]);
    let frames = state.file().frames as i64;
    state.viewport = WaveformViewport {
        start: (frames as f32 * 0.2).round() as i64,
        end: (frames as f32 * 0.6).round() as i64,
    };
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 40.0)));

    assert_pointer_location_output(output);
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
fn primary_drag_three_pixels_paints_playmark_selection_preview() {
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

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(drag))
            .is_none(),
        "creation drag should paint a live selection preview"
    );

    let live_selection = widget
        .live_selection_preview
        .expect("live playmark selection preview")
        .selection;
    assert!((live_selection.start() - 0.2).abs() < f32::EPSILON);
    assert!((live_selection.end() - 0.215).abs() < f32::EPSILON);

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(drag, PointerButton::Primary, Default::default()),
        )
        .expect("release should commit the playmark range")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    let selection = state.play_selection().expect("playmark selection");
    assert!((selection.start() - 0.2).abs() < f32::EPSILON);
    assert!((selection.end() - 0.215).abs() < f32::EPSILON);
}

#[test]
fn created_selection_drag_preview_survives_widget_rebuild_after_press() {
    for (button, kind) in [
        (PointerButton::Primary, WaveformSelectionKind::Play),
        (PointerButton::Secondary, WaveformSelectionKind::Edit),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        let mut previous = waveform_widget_for_state(&state);
        let bounds = Rect::from_size(200.0, 80.0);
        let press = Point::new(40.0, 40.0);
        let drag = Point::new(80.0, 40.0);

        let begin = previous
            .handle_input(
                bounds,
                WidgetInput::pointer_press(press, button, Default::default()),
            )
            .expect("press should begin a selection drag")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        state.apply_interaction(begin);

        let mut current = waveform_widget_for_state(&state);
        Widget::synchronize_from_previous(&mut current, &previous);
        assert!(
            current
                .handle_input(bounds, WidgetInput::pointer_move(drag))
                .is_none(),
            "rebuilt widget should retain the selection anchor without reducer output"
        );

        let preview = current
            .live_selection_preview
            .expect("rebuilt widget should paint a live selection preview");
        assert_eq!(preview.kind, kind);
        assert!((preview.selection.start() - 0.2).abs() < f32::EPSILON);
        assert!((preview.selection.end() - 0.4).abs() < f32::EPSILON);
    }
}

#[test]
fn resized_playmark_preview_survives_widget_rebuild_after_press() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut previous = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = previous
        .handle_input(bounds, WidgetInput::primary_press(Point::new(120.0, 8.0)))
        .expect("press should begin resizing the playmark")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        begin,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::End,
            visible_ratio: 0.6,
        }
    );
    state.apply_interaction(begin);

    let mut current = waveform_widget_for_state(&state);
    Widget::synchronize_from_previous(&mut current, &previous);
    let preview = current
        .live_selection_preview
        .expect("rebuilt widget should immediately keep the playmark resize preview");

    assert_eq!(preview.kind, WaveformSelectionKind::Play);
    assert_eq!(
        preview.selection,
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
    );
}

#[test]
fn playmark_drag_suppresses_duplicate_live_updates_inside_same_step() {
    let mut state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(40.0, 40.0)))
        .expect("primary press should begin playmark selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(43.0, 40.0)))
            .is_none(),
        "first crossed step should paint a local live selection preview"
    );
    let first_selection = widget
        .live_selection_preview
        .expect("first live selection preview")
        .selection;

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(43.2, 40.0)))
            .is_none(),
        "moves that quantize to the same visible step should not churn updates"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(first_selection),
        "sub-step motion should not churn live selection preview geometry"
    );

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(43.4, 40.0)))
            .is_none(),
        "same one-pixel preview step should still avoid reducer work"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(first_selection),
        "same preview step should not churn live selection preview geometry"
    );

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(44.0, 40.0)))
            .is_none(),
        "next crossed pixel should still paint locally"
    );
    assert_ne!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(first_selection),
        "crossing a preview step should update the live preview geometry"
    );
}

#[test]
fn moved_selection_drag_preview_survives_widget_rebuild_after_press() {
    for (kind, button, press, drag) in [
        (
            WaveformSelectionKind::Play,
            PointerButton::Primary,
            Point::new(80.0, 3.0),
            Point::new(100.0, 3.0),
        ),
        (
            WaveformSelectionKind::Edit,
            PointerButton::Primary,
            Point::new(60.0, 3.0),
            Point::new(80.0, 3.0),
        ),
        (
            WaveformSelectionKind::Edit,
            PointerButton::Secondary,
            Point::new(60.0, 3.0),
            Point::new(80.0, 3.0),
        ),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        match kind {
            WaveformSelectionKind::Play => {
                state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
                state.play_mark_ratio = Some(0.2);
            }
            WaveformSelectionKind::Edit => {
                state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
                state.edit_mark_ratio = Some(0.2);
            }
        }
        let mut previous = waveform_widget_for_state(&state);
        let bounds = Rect::from_size(200.0, 80.0);

        let begin = previous
            .handle_input(
                bounds,
                WidgetInput::pointer_press(press, button, Default::default()),
            )
            .expect("press should begin moving the existing selection")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        assert_eq!(
            begin,
            WaveformInteraction::BeginSelectionMove {
                kind,
                visible_ratio: match kind {
                    WaveformSelectionKind::Play => 0.4,
                    WaveformSelectionKind::Edit => 0.3,
                }
            }
        );
        state.apply_interaction(begin);

        let mut current = waveform_widget_for_state(&state);
        Widget::synchronize_from_previous(&mut current, &previous);
        assert!(
            current
                .handle_input(bounds, WidgetInput::pointer_move(drag))
                .is_none(),
            "rebuilt widget should retain the move anchor without reducer output"
        );

        let preview = current
            .live_selection_preview
            .expect("rebuilt widget should paint the moved selection preview");
        assert_eq!(preview.kind, kind);
        assert!((preview.selection.start() - 0.3).abs() < f32::EPSILON);
        assert!((preview.selection.end() - 0.7).abs() < f32::EPSILON);
    }
}

#[test]
fn moved_selection_drag_preview_uses_original_baseline_after_live_update() {
    for (kind, button) in [
        (WaveformSelectionKind::Play, PointerButton::Primary),
        (WaveformSelectionKind::Edit, PointerButton::Secondary),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        state.viewport = super::WaveformViewport {
            start: 12_000,
            end: 36_000,
        };
        let baseline = wavecrate::selection::SelectionRange::new(0.35, 0.55);
        match kind {
            WaveformSelectionKind::Play => {
                state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.35, 0.55));
                state.play_mark_ratio = Some(0.35);
            }
            WaveformSelectionKind::Edit => {
                state.edit_selection = Some(baseline);
                state.edit_mark_ratio = Some(0.35);
            }
        }
        let bounds = Rect::from_size(200.0, 80.0);
        let press = Point::new(60.0, 3.0);
        let first_drag = Point::new(80.0, 3.0);
        let second_drag = Point::new(90.0, 3.0);

        let mut initial = waveform_widget_for_state(&state);
        let begin = initial
            .handle_input(
                bounds,
                WidgetInput::pointer_press(press, button, Default::default()),
            )
            .expect("press should begin moving selection")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        assert_eq!(
            begin,
            WaveformInteraction::BeginSelectionMove {
                kind,
                visible_ratio: 0.3,
            }
        );
        state.apply_interaction(begin);

        let mut first = waveform_widget_for_state(&state);
        Widget::synchronize_from_previous(&mut first, &initial);
        assert!(
            first
                .handle_input(bounds, WidgetInput::pointer_move(first_drag))
                .is_none(),
            "first drag should paint the move preview locally"
        );

        let mut second = first;
        assert!(
            second
                .handle_input(bounds, WidgetInput::pointer_move(second_drag))
                .is_none(),
            "second drag should keep the move preview paint-only"
        );

        let preview = second
            .live_selection_preview
            .expect("rebuilt widget should keep previewing from original baseline");
        assert_eq!(preview.kind, kind);
        assert!(
            (preview.selection.start() - 0.425).abs() < 0.0001,
            "{kind:?} preview should apply the total drag delta once"
        );
        assert!(
            (preview.selection.end() - 0.625).abs() < 0.0001,
            "{kind:?} preview should apply the total drag delta once"
        );

        let finish = second
            .handle_input(
                bounds,
                WidgetInput::pointer_release(second_drag, button, Default::default()),
            )
            .expect("release should finish the move")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        state.apply_interaction(finish);

        let committed = match kind {
            WaveformSelectionKind::Play => state.play_selection(),
            WaveformSelectionKind::Edit => state.edit_selection(),
        }
        .expect("selection should remain after release");
        assert!((committed.start() - preview.selection.start()).abs() < 0.0001);
        assert!((committed.end() - preview.selection.end()).abs() < 0.0001);
    }
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

        assert!(
            widget
                .handle_input(bounds, WidgetInput::pointer_move(drag))
                .is_none(),
            "three-pixel creation drag should paint live without reducer output"
        );
        let live_selection = widget
            .live_selection_preview
            .expect("selection preview should update immediately after click slop")
            .selection;
        assert!(
            live_selection.width() > 0.0,
            "{kind:?} live selection should have non-zero width"
        );

        let finish = widget
            .handle_input(
                bounds,
                WidgetInput::pointer_release(drag, button, Default::default()),
            )
            .expect("release should commit range selection")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        state.apply_interaction(finish);

        let selection = match kind {
            WaveformSelectionKind::Play => state.play_selection(),
            WaveformSelectionKind::Edit => state.edit_selection(),
        }
        .expect("selection should commit on release");
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
fn playmark_resize_motion_updates_live_until_release() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(120.0, 8.0)))
        .expect("playmark resize should begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(160.0, 8.0)))
            .is_none(),
        "resize motion should paint a live selection preview"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.8)),
        "drag motion should paint app playmark geometry live"
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(160.0, 8.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("playmark resize should finish")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.8))
    );
}

#[test]
fn playmark_right_resize_updates_when_returning_through_original_handle() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(120.0, 8.0)))
        .expect("right-edge resize should begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        begin,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::End,
            visible_ratio: 0.6,
        }
    );
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(160.0, 8.0)))
            .is_none(),
        "dragging away should update the right edge preview"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.8))
    );

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(121.0, 8.0)))
            .is_none(),
        "returning through the original handle must still update the preview"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.605))
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(121.0, 8.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("right-edge resize should finish at the live preview")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);
    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.605))
    );
}

#[test]
fn playmark_left_resize_updates_when_returning_through_original_handle() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(40.0, 8.0)))
        .expect("left-edge resize should begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    assert_eq!(
        begin,
        WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::Start,
            visible_ratio: 0.2,
        }
    );
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(0.0, 8.0)))
            .is_none(),
        "dragging away should update the left edge preview"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.0, 0.6))
    );

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(41.0, 8.0)))
            .is_none(),
        "returning through the original handle must still update the preview"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.205, 0.6))
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(41.0, 8.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("left-edge resize should finish at the live preview")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);
    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.205, 0.6))
    );
}

#[test]
fn playmark_resize_crosses_opposite_edge_without_dead_zone() {
    for (press, drag, edge, expected_visible_ratio, expected_selection) in [
        (
            Point::new(120.0, 8.0),
            Point::new(20.0, 8.0),
            WaveformSelectionEdge::End,
            0.1,
            wavecrate::selection::SelectionRange::new(0.1, 0.2),
        ),
        (
            Point::new(40.0, 8.0),
            Point::new(160.0, 8.0),
            WaveformSelectionEdge::Start,
            0.8,
            wavecrate::selection::SelectionRange::new(0.6, 0.8),
        ),
    ] {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        state.play_mark_ratio = Some(0.2);
        let mut widget = waveform_widget_for_state(&state);
        let bounds = Rect::from_size(200.0, 80.0);

        let begin = widget
            .handle_input(bounds, WidgetInput::primary_press(press))
            .expect("playmark resize should begin")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        assert_eq!(
            begin,
            WaveformInteraction::BeginSelectionResize {
                kind: WaveformSelectionKind::Play,
                edge,
                visible_ratio: press.x / bounds.width(),
            }
        );
        state.apply_interaction(begin);
        widget.active_drag_kind = state.active_drag_kind();

        assert!(
            widget
                .handle_input(bounds, WidgetInput::pointer_move(drag))
                .is_none(),
            "crossing the opposite edge should paint a live preview"
        );
        assert_eq!(
            expected_visible_ratio,
            drag.x / bounds.width(),
            "test expectation should match the pointer ratio"
        );
        assert_eq!(
            widget
                .live_selection_preview
                .map(|preview| preview.selection),
            Some(expected_selection)
        );

        let finish = widget
            .handle_input(
                bounds,
                WidgetInput::pointer_release(drag, PointerButton::Primary, Default::default()),
            )
            .expect("crossed-edge resize should finish")
            .typed_copied::<WaveformInteraction>()
            .expect("waveform interaction");
        state.apply_interaction(finish);
        assert_eq!(state.play_selection(), Some(expected_selection));
    }
}

#[test]
fn zoomed_playmark_resize_preview_matches_committed_transform() {
    let mut state = WaveformState::synthetic_for_tests();
    state.viewport = super::WaveformViewport {
        start: 12_000,
        end: 36_000,
    };
    let fixed = state.absolute_ratio_from_visible(0.25);
    let initial_end = state.absolute_ratio_from_visible(0.5);
    let released = state.absolute_ratio_from_visible(0.75);
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(
        fixed,
        initial_end,
    ));
    state.play_mark_ratio = Some(fixed);
    let mut previous = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = previous
        .handle_input(bounds, WidgetInput::primary_press(Point::new(100.0, 8.0)))
        .expect("press should begin zoomed playmark resize")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    let mut current = waveform_widget_for_state(&state);
    Widget::synchronize_from_previous(&mut current, &previous);

    assert!(
        current
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(150.0, 8.0)))
            .is_none(),
        "resize motion should update preview without reducer output"
    );
    let preview = current
        .live_selection_preview
        .expect("zoomed resize should paint live preview");
    assert_eq!(
        preview.selection,
        wavecrate::selection::SelectionRange::new(fixed, released)
    );

    let finish = current
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(150.0, 8.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("release should finish zoomed resize")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(fixed, released))
    );
}

#[test]
fn playmark_move_motion_updates_live_until_release() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(bounds, WidgetInput::primary_press(Point::new(80.0, 3.0)))
        .expect("playmark move should begin")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(110.0, 3.0)))
            .is_none(),
        "move motion should update the live preview without reducer output"
    );
    assert_eq!(
        widget
            .live_selection_preview
            .map(|preview| preview.selection),
        Some(wavecrate::selection::SelectionRange::new(0.35, 0.75)),
        "drag motion should paint the moved playmark live"
    );

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(110.0, 3.0),
                PointerButton::Primary,
                Default::default(),
            ),
        )
        .expect("playmark move should finish")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert_eq!(
        state.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.35, 0.75))
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
fn secondary_click_on_playmark_body_cancels_editmark_instead_of_context_menu() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.1, 0.15));
    state.edit_mark_ratio = Some(0.1);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let begin = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(
                Point::new(80.0, 40.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("secondary press should begin tentative edit selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(80.0, 40.0),
                PointerButton::Secondary,
                Default::default(),
            ),
        )
        .expect("secondary release should finish edit click")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    assert_eq!(state.edit_selection(), None);
    assert_eq!(state.edit_mark_ratio(), None);
}

#[test]
fn secondary_drag_from_playmark_body_paints_edit_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    let press = Point::new(80.0, 40.0);
    let drag = Point::new(150.0, 40.0);

    let begin = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_press(press, PointerButton::Secondary, Default::default()),
        )
        .expect("secondary press should begin tentative edit selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(begin);
    widget.active_drag_kind = state.active_drag_kind();

    assert!(
        widget
            .handle_input(bounds, WidgetInput::pointer_move(drag))
            .is_none(),
        "secondary creation drag should update edit selection preview"
    );

    let live_selection = widget
        .live_selection_preview
        .expect("live edit selection preview")
        .selection;
    assert!((live_selection.start() - 0.4).abs() < f32::EPSILON);
    assert!((live_selection.end() - 0.75).abs() < f32::EPSILON);

    let finish = widget
        .handle_input(
            bounds,
            WidgetInput::pointer_release(drag, PointerButton::Secondary, Default::default()),
        )
        .expect("secondary release should commit edit selection")
        .typed_copied::<WaveformInteraction>()
        .expect("waveform interaction");
    state.apply_interaction(finish);

    let selection = state.edit_selection().expect("edit selection");
    assert!((selection.start() - 0.4).abs() < f32::EPSILON);
    assert!((selection.end() - 0.75).abs() < f32::EPSILON);
}

#[test]
fn editmark_selection_drag_does_not_churn_signal_preview() {
    let mut state = WaveformState::synthetic_for_tests();

    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });

    assert_eq!(
        state.edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6)),
        "editmark model selection should update live"
    );
    assert_eq!(
        signal_edit_selection_for_state(&state),
        None,
        "plain editmark drag should paint through the lightweight overlay"
    );

    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });

    assert_eq!(
        signal_edit_selection_for_state(&state),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6)),
        "finished editmark selection should feed the edit preview again"
    );
}

#[test]
fn edit_gain_drag_keeps_signal_preview_live() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);

    state.apply_interaction(WaveformInteraction::BeginEditGain { pointer_y: 20.0 });

    assert_eq!(
        state.active_drag_kind(),
        Some(WaveformActiveDragKind::EditGain)
    );
    assert_eq!(
        signal_edit_selection_for_state(&state),
        state.edit_selection(),
        "edit gain drags should keep the signal gain preview live"
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
