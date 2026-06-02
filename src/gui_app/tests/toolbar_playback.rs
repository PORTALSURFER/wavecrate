use super::*;

#[test]
fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
    for icon in [
        super::super::ToolbarIcon::FocusLoaded,
        super::super::ToolbarIcon::Loop,
        super::super::ToolbarIcon::Play,
        super::super::ToolbarIcon::Stop,
    ] {
        assert_eq!(
            super::super::toolbar_icon_color(true, false),
            radiant::prelude::Rgba8::new(238, 238, 238, 255)
        );
        assert_eq!(
            super::super::toolbar_icon_color(true, true),
            radiant::prelude::Rgba8::new(255, 160, 82, 255)
        );
        assert_eq!(
            super::super::toolbar_icon_color(false, false),
            radiant::prelude::Rgba8::new(145, 145, 145, 255)
        );
        let mut primitives = Vec::new();
        super::super::toolbar_icon_glyph(icon, true, false).append_paint(
            &mut primitives,
            101,
            Rect::from_size(28.0, 24.0),
        );
        assert!(
            primitives.iter().any(|primitive| primitive.svg().is_some()),
            "toolbar icon cache should produce a retained Radiant SVG"
        );
        let frame = super::super::toolbar_icon_button(101, icon, true, false)
            .view_frame_at_size_with_default_theme(Vector2::new(28.0, 24.0));
        assert!(
            frame.paint_plan.svgs().next().is_some(),
            "toolbar icon should paint as a retained Radiant SVG"
        );
    }
}

#[test]
fn toolbar_icon_button_routes_messages_through_radiant_builder() {
    for (icon, message) in [
        (
            super::super::ToolbarIcon::FocusLoaded,
            super::super::GuiMessage::FocusLoadedFile,
        ),
        (
            super::super::ToolbarIcon::Loop,
            super::super::GuiMessage::ToggleLoopPlayback,
        ),
    ] {
        assert_eq!(
            super::super::toolbar_icon_button(101, icon, true, false).view_dispatch_widget_output(
                101,
                radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate),
            ),
            Some(message)
        );
    }
}

#[test]
fn focus_loaded_toolbar_button_is_topmost_hit_target_and_paints_hover_feedback() {
    let state = GuiAppState::load_default().expect("default state loads");
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(super::super::TOOLBAR_FOCUS_LOADED_ID)
        .expect("focus-loaded toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(super::super::TOOLBAR_FOCUS_LOADED_ID),
        "focus-loaded button must be the topmost hit target at its painted icon"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(super::super::TOOLBAR_FOCUS_LOADED_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(super::super::TOOLBAR_FOCUS_LOADED_ID),
        "hovering the focus-loaded button should paint a visible accent overlay"
    );
}

#[test]
fn stop_toolbar_button_is_hit_target_and_paints_hover_while_playing() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.waveform = super::super::WaveformState::synthetic_for_tests();
    state.waveform.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(super::super::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(super::super::TOOLBAR_STOP_ID),
        "stop button must be the topmost hit target while playback is active"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(super::super::TOOLBAR_STOP_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(super::super::TOOLBAR_STOP_ID),
        "hovering the playing stop button should paint a visible accent overlay"
    );
    runtime.dispatch_primary_click(point);
    assert!(
        !runtime.bridge().state().waveform.is_playing(),
        "clicking the playing stop button should dispatch StopPlayback"
    );
}

#[test]
fn stop_toolbar_button_remains_available_for_loaded_idle_sample() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.waveform = super::super::WaveformState::synthetic_for_tests();
    assert!(!state.waveform.is_playing());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(super::super::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(super::super::TOOLBAR_STOP_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(super::super::TOOLBAR_STOP_ID)
    );
}

#[test]
fn playback_frame_uses_paint_only_when_only_playhead_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame();

    assert!(
        state.frame_can_use_paint_only(before),
        "playback-only frames should not force full surface reprojection"
    );
}

#[test]
fn playback_frame_repaints_surface_when_playback_state_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.waveform.stop_playback();

    assert!(
        !state.frame_can_use_paint_only(before),
        "stopping playback changes toolbar/status surface state and needs a full repaint"
    );
}

#[test]
fn playback_cursor_paints_as_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);

    assert!(
        !frame
            .paint_plan
            .fill_rects_for_widget(super::super::WAVEFORM_WIDGET_ID)
            .any(|fill| { fill.color.r == 71 && fill.color.g == 220 && fill.color.b == 255 }),
        "live playback cursor should not be baked into the cached surface"
    );

    let mut primitives = Vec::new();
    runtime.bridge_mut().state_mut().paint_playback_overlay(
        TransientOverlayContext::new(
            &frame.paint_plan,
            Vector2::new(900.0, 620.0),
            Duration::ZERO,
        ),
        &mut primitives,
    );

    assert!(
        primitives
            .iter()
            .filter_map(|primitive| primitive.fill_rect())
            .any(|fill| {
                fill.widget_id == super::super::WAVEFORM_WIDGET_ID
                    && fill.color.r == 71
                    && fill.color.g == 220
                    && fill.color.b == 255
            }),
        "paint-only playback overlay should append the live cursor"
    );
}
