use super::*;

fn runtime_overlay_plan(widget: &WaveformWidget, bounds: Rect) -> SurfacePaintPlan {
    let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
    widget.append_runtime_overlay_paint(
        &mut plan.primitives,
        bounds,
        &radiant::layout::LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    plan
}

fn assert_no_white_hover_border(plan: &SurfacePaintPlan) {
    assert!(
        !plan.stroke_rects().any(|stroke| {
            (stroke.color.r, stroke.color.g, stroke.color.b) == (255, 255, 255)
                && stroke.color.a >= 200
        }),
        "handle hover should use a brighter colored fill, not a white border"
    );
}

#[test]
fn overlay_paint_projects_play_edit_and_playhead_markers() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_playback(0.125);
    state.set_playhead_ratio(0.25);
    state.stop_playback();
    state.set_playhead_ratio(0.25);
    state.edit_mark_ratio = Some(0.375);

    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.125).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (204, 255, 255, 245)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.375).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b) == (82, 168, 255)
            && fill.color.a == 230
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.25).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b) == (71, 220, 255)
            && fill.color.a == 245
    }));
}

#[test]
fn playhead_cursor_paints_pixel_stable_rect_when_progress_is_subpixel() {
    let mut state = WaveformState::synthetic_for_tests();
    state.set_playhead_ratio(0.12345);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    let playhead = fill_rects(&plan)
        .into_iter()
        .find(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (71, 220, 255, 245)
        })
        .expect("playhead fill paints");
    assert_eq!(playhead.rect.width(), 2.0);
    assert_eq!(playhead.rect.min.x.fract(), 0.0);
    assert_eq!(playhead.rect.max.x.fract(), 0.0);
}

#[test]
fn playhead_cursor_paints_while_playing_small_playmark_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.500, 0.505));
    state.start_playback(0.500);
    state.set_playhead_ratio(0.503);

    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    let playhead = fill_rects(&plan)
        .into_iter()
        .find(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (71, 220, 255, 245)
        })
        .expect("playhead fill paints during playback");
    assert!((playhead.rect.center().x / 400.0 - 0.503).abs() < 0.01);
}

#[test]
fn hover_cursor_paints_thin_white_overlay_line() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(400.0, 80.0);
    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(100.0, 40.0)));
    assert_pointer_location_output(output);

    let plan = runtime_overlay_plan(&widget, bounds);

    let cursor = fill_rects(&plan)
        .into_iter()
        .find(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 255, 255, 210)
        })
        .expect("hover cursor fill paints");
    assert_eq!(cursor.rect.width(), 1.0);
    assert!((cursor.rect.center().x - 100.0).abs() < 1.0);
}

#[test]
fn playmark_slide_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 3.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 49.0).abs() < 0.001
            && (fill.rect.max.x - 111.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 7.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 255)
    }));
    assert!(!fills.iter().any(
        |fill| (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 255, 255, 210)
    ));
}

#[test]
fn playmark_resize_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(120.0, 8.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 116.5).abs() < 0.001
            && (fill.rect.max.x - 123.5).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 22.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 202, 112, 255)
    }));
    assert!(!fills.iter().any(|fill| {
        (fill.rect.min.x - 112.5).abs() < 0.001
            && (fill.rect.max.x - 127.5).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 22.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 180)
    }));
    assert_no_white_hover_border(&plan);
}

#[test]
fn playmark_export_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(118.0, 76.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 104.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 64.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 202, 112, 255)
    }));
    assert!(!fills.iter().any(|fill| {
        (fill.rect.min.x - 100.0).abs() < 0.001
            && (fill.rect.max.x - 124.0).abs() < 0.001
            && (fill.rect.min.y - 60.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 180)
    }));
    assert!(!fills.iter().any(
        |fill| (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 255, 255, 210)
    ));
    assert_no_white_hover_border(&plan);
}

#[test]
fn editmark_slide_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 3.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 49.0).abs() < 0.001
            && (fill.rect.max.x - 111.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 7.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
}

#[test]
fn editmark_bottom_resize_handles_paint_on_base_edit_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 36.5).abs() < 0.001
            && (fill.rect.max.x - 43.5).abs() < 0.001
            && (fill.rect.min.y - 58.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 190)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 116.5).abs() < 0.001
            && (fill.rect.max.x - 123.5).abs() < 0.001
            && (fill.rect.min.y - 58.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 190)
    }));
}

#[test]
fn editmark_bottom_resize_handle_hides_on_faded_side() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_out(0.25, 0.7),
    );
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(!fills.iter().any(|fill| {
        (fill.rect.min.x - 36.5).abs() < 0.001
            && (fill.rect.max.x - 43.5).abs() < 0.001
            && (fill.rect.min.y - 58.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 190)
    }));
    assert!(!fills.iter().any(|fill| {
        (fill.rect.min.x - 116.5).abs() < 0.001
            && (fill.rect.max.x - 123.5).abs() < 0.001
            && (fill.rect.min.y - 58.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 190)
    }));
}

#[test]
fn editmark_bottom_resize_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(120.0, 76.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 116.5).abs() < 0.001
            && (fill.rect.max.x - 123.5).abs() < 0.001
            && (fill.rect.min.y - 58.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
    assert_no_white_hover_border(&plan);
}

#[test]
fn editmark_gain_handle_hover_paints_bright_center_tab() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(80.0, 5.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);
    assert!(widget.hovered_edit_gain_handle);
    assert_eq!(widget.hovered_selection_handle, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 74.0).abs() < 0.001
            && (fill.rect.max.x - 86.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 10.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
}

#[test]
fn editmark_gain_handle_paints_on_base_edit_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 74.0).abs() < 0.001
            && (fill.rect.max.x - 86.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 10.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 225)
    }));
}

#[test]
fn edit_fade_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 5.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 55.0).abs() < 0.001
            && (fill.rect.max.x - 65.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 10.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
}

#[test]
fn edit_fade_outer_gain_handle_paints_at_current_gain_height() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.25)
            .with_fade_in_outer_gain(0.0),
    );
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 15.0).abs() < 0.001
            && (fill.rect.max.x - 25.0).abs() < 0.001
            && (fill.rect.min.y - 35.0).abs() < 0.001
            && (fill.rect.max.y - 45.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 205)
    }));
}

#[test]
fn edit_fade_outer_gain_handle_hover_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.25, 0.2)
            .with_fade_in_mute(0.25),
    );
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 5.0)));
    assert_pointer_location_output(output);
    assert_eq!(
        widget.hovered_edit_fade_outer_gain_handle,
        Some(WaveformEditFadeOuterGainHandle::In)
    );

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 15.0).abs() < 0.001
            && (fill.rect.max.x - 25.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 10.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
}

#[test]
fn play_start_marker_is_hidden_at_sample_start() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_playback(0.0);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    assert!(
        !fill_rects(&plan).iter().any(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (204, 255, 255, 245)
        }),
        "play-start marker should be implicit when playback starts at sample head"
    );
}

#[test]
fn play_start_marker_paints_cyan_white_when_start_deviates_from_sample_start() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_playback(0.125);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    assert!(fill_rects(&plan).iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.125).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (204, 255, 255, 245)
    }));
}

#[test]
fn play_start_marker_paints_even_when_play_selection_exists() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.start_playback(0.4);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(400.0, 80.0));

    assert!(fill_rects(&plan).iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.4).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (204, 255, 255, 245)
    }));
}

#[test]
fn beat_guides_paint_internal_lines_inside_play_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state_with_beat_guides(&state, true, 4);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let guides = fill_rects(&plan)
        .into_iter()
        .filter(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 214, 188, 170)
        })
        .collect::<Vec<_>>();

    assert_eq!(guides.len(), 3);
    for expected_x in [60.0, 80.0, 100.0] {
        assert!(
            guides.iter().any(|fill| {
                (fill.rect.center().x - expected_x).abs() < 0.01
                    && (fill.rect.min.y - 11.0).abs() < 0.01
                    && (fill.rect.max.y - 69.0).abs() < 0.01
            }),
            "expected beat guide at x={expected_x}, got {guides:?}"
        );
    }
}

#[test]
fn beat_guides_paint_internal_lines_inside_edit_selection() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state_with_beat_guides(&state, true, 4);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let guides = fill_rects(&plan)
        .into_iter()
        .filter(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 214, 188, 170)
        })
        .collect::<Vec<_>>();

    assert_eq!(guides.len(), 3);
    for expected_x in [60.0, 80.0, 100.0] {
        assert!(
            guides.iter().any(|fill| {
                (fill.rect.center().x - expected_x).abs() < 0.01
                    && (fill.rect.min.y - 11.0).abs() < 0.01
                    && (fill.rect.max.y - 69.0).abs() < 0.01
            }),
            "expected beat guide at x={expected_x}, got {guides:?}"
        );
    }
}

#[test]
fn beat_guides_do_not_paint_when_toggle_is_off() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.3, 0.7));
    let widget = waveform_widget_for_state_with_beat_guides(&state, false, 4);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    assert!(!fill_rects(&plan).iter().any(|fill| {
        (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 214, 188, 170)
    }));
}

#[test]
fn selection_range_projects_visible_ratios_inside_viewport() {
    let mut state = WaveformState::synthetic_for_tests();
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.25,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection {
        visible_ratio: 0.75,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.75,
    });
    let widget = waveform_widget_for_state(&state);
    let range = widget
        .visible_normalized_range_for_selection(state.edit_selection())
        .expect("selection range");

    assert!((range.start_fraction() - 0.25).abs() < 0.001);
    assert!((range.end_fraction() - 0.75).abs() < 0.001);
}

#[test]
fn selection_fill_paints_as_overlay_widget_rects() {
    let mut state = WaveformState::synthetic_for_tests();
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
    state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    assert!(
        plan.gpu_surfaces().next().is_none(),
        "ordinary waveform overlay widget must not emit the GPU waveform"
    );
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 40.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 230)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 120.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 230)
    }));
}

#[test]
fn extracted_ranges_paint_as_gray_waveform_overlays() {
    let mut state = WaveformState::synthetic_for_tests();
    state
        .extracted_ranges
        .push(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (156, 160, 168, 108)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 2.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (206, 211, 219, 225)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 78.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (206, 211, 219, 225)
    }));
}

#[test]
fn static_range_overlays_pause_while_playmark_selection_drag_is_active() {
    let mut state = WaveformState::synthetic_for_tests();
    state
        .extracted_ranges
        .push(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![wavecrate::selection::SelectionRange::new(0.3, 0.7)]);
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.4, 0.8));
    state.play_mark_ratio = Some(0.4);

    let mut widget = waveform_widget_for_state(&state);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.live_selection_preview = Some(LiveSelectionPreview {
        kind: WaveformSelectionKind::Play,
        selection: wavecrate::selection::SelectionRange::new(0.4, 0.8),
    });
    let bounds = Rect::from_size(200.0, 80.0);
    let plan = widget.paint_plan_with_defaults(bounds);

    let fills = fill_rects(&plan);
    assert!(
        !fills.iter().any(|fill| {
            matches!(
                (fill.color.r, fill.color.g, fill.color.b, fill.color.a),
                (156, 160, 168, 108) | (114, 235, 184, 54)
            )
        }),
        "static extracted/similar overlays should not paint during live playmark drags"
    );
    let runtime_plan = runtime_overlay_plan(&widget, bounds);
    let runtime_fills = fill_rects(&runtime_plan);
    assert!(
        runtime_fills.iter().any(|fill| {
            (fill.rect.min.x - 80.0).abs() < 0.001
                && (fill.rect.max.x - 160.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
        }),
        "the live playmark selection itself should keep painting"
    );
}

#[test]
fn committed_selection_paint_pauses_while_selection_preview_is_live() {
    let mut state = WaveformState::synthetic_for_tests();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.play_mark_ratio = Some(0.2);
    let mut widget = waveform_widget_for_state(&state);
    widget.active_drag_kind = Some(WaveformActiveDragKind::SelectionResize(
        WaveformSelectionKind::Play,
        WaveformSelectionEdge::End,
    ));

    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(
        !fills.iter().any(|fill| {
            (fill.rect.min.x - 40.0).abs() < 0.001
                && (fill.rect.max.x - 120.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
        }),
        "committed play selection should not repaint under the live drag preview"
    );
}

#[test]
fn beat_guides_do_not_paint_during_live_selection_preview() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state_with_beat_guides(&state, true, 16);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.live_selection_preview = Some(LiveSelectionPreview {
        kind: WaveformSelectionKind::Play,
        selection: wavecrate::selection::SelectionRange::new(0.2, 0.6),
    });

    let plan = runtime_overlay_plan(&widget, Rect::from_size(200.0, 80.0));
    let fills = fill_rects(&plan);

    assert!(
        fills.iter().all(
            |fill| (fill.color.r, fill.color.g, fill.color.b, fill.color.a) != (255, 214, 188, 170)
        ),
        "beat guides should wait for drag release instead of painting every pointer update"
    );
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 40.0).abs() < 0.001
                && (fill.rect.max.x - 120.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
        }),
        "the live selection preview should still paint"
    );
}

#[test]
fn active_drag_widget_props_do_not_clone_static_range_overlays() {
    let mut state = WaveformState::synthetic_for_tests();
    state
        .extracted_ranges
        .push(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![wavecrate::selection::SelectionRange::new(0.3, 0.7)]);
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.4, 0.8));
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.4,
    });

    let props = WaveformWidgetProps::from_state(&state, false, 4);

    assert_eq!(props.static_range_overlay_counts(), (0, 0));
    assert_eq!(
        props.active_drag_kind,
        Some(WaveformActiveDragKind::Selection(
            WaveformSelectionKind::Play
        ))
    );
}

#[test]
fn live_selection_preview_paints_in_runtime_overlay() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.live_selection_preview = Some(LiveSelectionPreview {
        kind: WaveformSelectionKind::Play,
        selection: wavecrate::selection::SelectionRange::new(0.2, 0.6),
    });

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
    }));
}

#[test]
fn live_playmark_preview_paints_full_interactive_chrome() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Play,
    ));
    widget.live_selection_preview = Some(LiveSelectionPreview {
        kind: WaveformSelectionKind::Play,
        selection: wavecrate::selection::SelectionRange::new(0.2, 0.6),
    });

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
    }));
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 49.0).abs() < 0.001
                && (fill.rect.max.x - 111.0).abs() < 0.001
                && (fill.rect.min.y - 0.0).abs() < 0.001
                && (fill.rect.max.y - 7.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 185)
        }),
        "live playmark drag should keep the move handle visible"
    );
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 104.0).abs() < 0.001
                && (fill.rect.max.x - 120.0).abs() < 0.001
                && (fill.rect.min.y - 64.0).abs() < 0.001
                && (fill.rect.max.y - 80.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 235)
        }),
        "live playmark drag should keep the export handle visible"
    );
}

#[test]
fn live_editmark_preview_paints_full_interactive_chrome() {
    let state = WaveformState::synthetic_for_tests();
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);
    widget.active_drag_kind = Some(WaveformActiveDragKind::Selection(
        WaveformSelectionKind::Edit,
    ));
    widget.live_selection_preview = Some(LiveSelectionPreview {
        kind: WaveformSelectionKind::Edit,
        selection: wavecrate::selection::SelectionRange::new(0.2, 0.6),
    });

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);

    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 46)
    }));
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 49.0).abs() < 0.001
                && (fill.rect.max.x - 111.0).abs() < 0.001
                && (fill.rect.min.y - 0.0).abs() < 0.001
                && (fill.rect.max.y - 7.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 180)
        }),
        "live editmark drag should keep the move handle visible"
    );
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 36.5).abs() < 0.001
                && (fill.rect.max.x - 43.5).abs() < 0.001
                && (fill.rect.min.y - 58.0).abs() < 0.001
                && (fill.rect.max.y - 80.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 190)
        }),
        "live editmark drag should keep resize handles visible"
    );
    assert!(
        fills.iter().any(|fill| {
            (fill.rect.min.x - 74.0).abs() < 0.001
                && (fill.rect.max.x - 86.0).abs() < 0.001
                && (fill.rect.min.y - 0.0).abs() < 0.001
                && (fill.rect.max.y - 10.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 225)
        }),
        "live editmark drag should keep the gain handle visible"
    );
}

#[test]
fn similar_sections_paint_as_green_waveform_overlays() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![wavecrate::selection::SelectionRange::new(0.2, 0.6)]);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (114, 235, 184, 54)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 2.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (155, 255, 218, 210)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 78.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (155, 255, 218, 210)
    }));
}

#[test]
fn similar_section_hover_paints_brighter_runtime_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_similar_sections(wavecrate::selection::SelectionRange::new(0.1, 0.2));
    state.finish_similar_sections_scan(vec![wavecrate::selection::SelectionRange::new(0.2, 0.6)]);
    let mut widget = waveform_widget_for_state(&state);
    let bounds = Rect::from_size(200.0, 80.0);

    let output = widget.handle_input(bounds, WidgetInput::pointer_move(Point::new(80.0, 40.0)));
    assert_pointer_location_output(output);
    assert_eq!(widget.hover_cursor_ratio, None);

    let plan = runtime_overlay_plan(&widget, bounds);
    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (156, 255, 218, 92)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 0.0).abs() < 0.001
            && (fill.rect.max.y - 2.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (219, 255, 240, 255)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.rect.min.y - 78.0).abs() < 0.001
            && (fill.rect.max.y - 80.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (219, 255, 240, 255)
    }));
}

#[test]
fn edit_selection_paints_start_and_end_boundary_lines() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 40.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 230)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 120.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 230)
    }));
}

#[test]
fn edit_selection_flash_paints_bright_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.flash_edit_selection();
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let fills = fill_rects(&plan);
    assert!(fills.iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 118)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 40.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x - 120.0).abs() < 1.0
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 255)
    }));
}

#[test]
fn copied_file_flash_paints_waveform_confirmation_overlay() {
    let mut state = WaveformState::synthetic_for_tests();
    state.flash_copied_file();
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    assert!(fill_rects(&plan).iter().any(|fill| {
        fill.rect == Rect::from_size(200.0, 80.0)
            && fill.color == radiant::gui::types::Rgba8::new(255, 174, 89, 46)
    }));

    state.apply_interaction(WaveformInteraction::Frame);
    assert!(state.copy_flash_frames() > 0);
}

#[test]
fn edit_selection_denied_flash_paints_red_twice() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    state.flash_denied_selection(WaveformSelectionKind::Edit);

    let initial_plan =
        waveform_widget_for_state(&state).paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    assert!(contains_denied_selection_fill(&initial_plan));

    for _ in 0..6 {
        state.apply_interaction(WaveformInteraction::Frame);
    }
    let between_plan =
        waveform_widget_for_state(&state).paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    assert!(!contains_denied_selection_fill(&between_plan));

    for _ in 0..6 {
        state.apply_interaction(WaveformInteraction::Frame);
    }
    let second_plan =
        waveform_widget_for_state(&state).paint_plan_with_defaults(Rect::from_size(200.0, 80.0));
    assert!(contains_denied_selection_fill(&second_plan));
}

#[test]
fn edit_fade_curve_paints_s_curve_shape_as_polyline() {
    let mut state = WaveformState::synthetic_for_tests();
    let selection = wavecrate::selection::SelectionRange::new(0.2, 0.6)
        .with_gain(0.5)
        .with_fade_in(0.5, 0.8)
        .with_fade_out(0.25, 0.0);
    state.edit_selection = Some(selection);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let curves = stroke_polylines(&plan)
        .into_iter()
        .filter(|stroke| {
            (
                stroke.color.r,
                stroke.color.g,
                stroke.color.b,
                stroke.color.a,
            ) == (82, 168, 255, 225)
                && stroke.width == 2.0
                && stroke.points.len() >= 10
        })
        .collect::<Vec<_>>();
    assert_eq!(
        curves.len(),
        2,
        "expected leading and trailing fade curve strokes"
    );

    let leading = curves
        .iter()
        .find(|stroke| {
            let points = stroke.points.as_ref();
            (points[0].x - 40.0).abs() < 0.001
                && (points.last().expect("last leading fade point").x - 80.0).abs() < 0.001
        })
        .expect("leading fade curve stroke");
    let leading_points = leading.points.as_ref();
    let leading_bend = leading_points[2];
    let leading_expected_y = 80.0 - 80.0 * wavecrate::selection::fade_curve_value(0.2, 0.8);
    assert!((leading_points[0].y - 80.0).abs() < 0.001);
    assert!((leading_points.last().expect("last leading fade point").y - 0.0).abs() < 0.001);
    assert!((leading_bend.x - 48.0).abs() < 0.001);
    assert!((leading_bend.y - leading_expected_y).abs() < 0.001);
    assert!(
        leading_bend.y > 70.0,
        "high-curve fade-in should paint the eased bend, not a straight ramp"
    );

    let trailing = curves
        .iter()
        .find(|stroke| {
            let points = stroke.points.as_ref();
            (points[0].x - 100.0).abs() < 0.001
                && (points.last().expect("last trailing fade point").x - 120.0).abs() < 0.001
        })
        .expect("trailing fade curve stroke");
    let trailing_points = trailing.points.as_ref();
    let trailing_mid = trailing_points[5];
    let trailing_expected_y =
        80.0 - 80.0 * (1.0 - wavecrate::selection::fade_curve_value(0.5, 0.0));
    assert!((trailing_points[0].y - 0.0).abs() < 0.001);
    assert!((trailing_points.last().expect("last trailing fade point").y - 80.0).abs() < 0.001);
    assert!((trailing_mid.x - 110.0).abs() < 0.001);
    assert!((trailing_mid.y - trailing_expected_y).abs() < 0.001);
}

fn contains_denied_selection_fill(plan: &SurfacePaintPlan) -> bool {
    fill_rects(plan).iter().any(|fill| {
        (fill.rect.min.x - 40.0).abs() < 0.001
            && (fill.rect.max.x - 120.0).abs() < 0.001
            && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 72, 82, 130)
    })
}

#[test]
fn edit_fade_curve_stays_inside_selection_when_outer_mute_extends_handles() {
    let mut state = WaveformState::synthetic_for_tests();
    let selection = wavecrate::selection::SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.2)
        .with_fade_in_mute(0.25)
        .with_fade_out(0.25, 0.7)
        .with_fade_out_mute(0.25);
    state.edit_selection = Some(selection);
    let widget = waveform_widget_for_state(&state);
    let plan = widget.paint_plan_with_defaults(Rect::from_size(200.0, 80.0));

    let curves = stroke_polylines(&plan)
        .into_iter()
        .filter(|stroke| {
            (
                stroke.color.r,
                stroke.color.g,
                stroke.color.b,
                stroke.color.a,
            ) == (82, 168, 255, 225)
                && stroke.width == 2.0
                && stroke.points.len() >= 10
        })
        .collect::<Vec<_>>();
    assert_eq!(curves.len(), 2);

    assert!(curves.iter().any(|stroke| {
        let points = stroke.points.as_ref();
        (points[0].x - 40.0).abs() < 0.001
            && (points.last().expect("last leading fade point").x - 60.0).abs() < 0.001
    }));
    assert!(curves.iter().any(|stroke| {
        let points = stroke.points.as_ref();
        (points[0].x - 100.0).abs() < 0.001
            && (points.last().expect("last trailing fade point").x - 120.0).abs() < 0.001
    }));
}
