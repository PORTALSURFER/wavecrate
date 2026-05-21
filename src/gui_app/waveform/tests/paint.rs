use super::*;

#[test]
fn overlay_paint_projects_play_edit_and_playhead_markers() {
    let mut state = WaveformState::synthetic_for_tests();
    state.start_playback(0.125);
    state.set_playhead_ratio(0.25);
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Edit,
        visible_ratio: 0.375,
    });
    state.apply_interaction(WaveformInteraction::FinishSelection {
        visible_ratio: 0.375,
    });

    let widget = waveform_widget_for_state(&state);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );

    let fills = fill_rects(&primitives);
    assert!(fills.iter().any(|fill| {
        (fill.rect.center().x / 400.0 - 0.125).abs() < 0.01
            && (fill.color.r, fill.color.g, fill.color.b) == (255, 142, 92)
            && fill.color.a == 230
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
    state.start_playback(0.0);
    state.set_playhead_ratio(0.12345);
    let widget = waveform_widget_for_state(&state);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );

    let playhead = fill_rects(&primitives)
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
    let (start, end) = widget
        .visible_range_for_selection(state.edit_selection())
        .expect("selection range");

    assert!((start - 0.25).abs() < 0.001);
    assert!((end - 0.75).abs() < 0.001);
}

#[test]
fn selection_fill_paints_as_overlay_widget_rects() {
    let mut state = WaveformState::synthetic_for_tests();
    state.apply_interaction(WaveformInteraction::BeginSelection {
        kind: WaveformSelectionKind::Play,
        visible_ratio: 0.2,
    });
    state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
    let widget = waveform_widget_for_state(&state);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );

    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_))),
        "ordinary waveform overlay widget must not emit the GPU waveform"
    );
    let fills = fill_rects(&primitives);
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
fn edit_selection_paints_start_and_end_boundary_lines() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
    let widget = waveform_widget_for_state(&state);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );

    let fills = fill_rects(&primitives);
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
fn edit_fade_curve_paints_volume_trace_as_overlay_rects() {
    let mut state = WaveformState::synthetic_for_tests();
    state.edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.6)
            .with_fade_in(0.5, 0.8)
            .with_fade_out(0.25, 0.0),
    );
    let widget = waveform_widget_for_state(&state);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );

    let curve_points = fill_rects(&primitives)
        .into_iter()
        .filter(|fill| {
            (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 225)
        })
        .count();
    assert!(
        curve_points >= 16,
        "expected visible fade curve trace points, got {curve_points}"
    );
}
