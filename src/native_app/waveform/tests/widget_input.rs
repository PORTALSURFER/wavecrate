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
                Point::new(80.0, 3.0),
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
            visible_ratio: 0.4
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
