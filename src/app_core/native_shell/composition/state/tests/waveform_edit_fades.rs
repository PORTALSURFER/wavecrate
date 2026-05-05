use super::*;

#[test]
fn waveform_motion_overlay_hides_edit_fade_vertical_bars() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_out_start_milli = Some(690);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let span = f32::from(edit_selection.end_milli - edit_selection.start_milli).max(1.0);
    let handle_in_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(320u16 - edit_selection.start_milli) / span));
    let handle_out_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(690u16 - edit_selection.start_milli) / span));

    let has_in_bar = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.min.y <= edit_rect.min.y
                    && rect.rect.max.y >= edit_rect.max.y
                    && rect.rect.height() >= edit_rect.height()
                    && rect.rect.width() <= 8.0
        )
    });
    let has_out_bar = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.min.y <= edit_rect.min.y
                    && rect.rect.max.y >= edit_rect.max.y
                    && rect.rect.height() >= edit_rect.height()
                    && rect.rect.width() <= 8.0
        )
    });
    assert!(
        !has_in_bar,
        "top fade-in handle should not draw a full-height bar"
    );
    assert!(
        !has_out_bar,
        "top fade-out handle should not draw a full-height bar"
    );
}
#[test]
fn waveform_motion_overlay_draws_edit_fade_top_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_out_start_milli = Some(690);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let span = f32::from(edit_selection.end_milli - edit_selection.start_milli).max(1.0);
    let handle_in_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(320u16 - edit_selection.start_milli) / span));
    let handle_out_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(690u16 - edit_selection.start_milli) / span));

    let has_in_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.min.y == edit_rect.min.y
                    && rect.rect.height() < edit_rect.height()
        )
    });
    let has_out_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.min.y == edit_rect.min.y
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(has_in_tab, "expected top grab tab for fade-in handle");
    assert!(has_out_tab, "expected top grab tab for fade-out handle");
}

#[test]
fn waveform_motion_overlay_draws_square_edit_fade_top_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(495, 505);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(497);
    model.waveform.edit_fade_out_start_milli = Some(503);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let span = f32::from(edit_selection.end_milli - edit_selection.start_milli).max(1.0);
    let handle_in_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(497u16 - edit_selection.start_milli) / span));
    let handle_out_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(503u16 - edit_selection.start_milli) / span));

    let top_tab_for = |handle_x: f32| {
        frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect)
                    if rect.rect.min.x <= handle_x
                        && rect.rect.max.x >= handle_x
                        && rect.rect.min.y == edit_rect.min.y
                        && rect.rect.height() < edit_rect.height() =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .max_by(|left, right| {
                (left.width() * left.height())
                    .partial_cmp(&(right.width() * right.height()))
                    .expect("tab area should be comparable")
            })
    };

    let fade_in_tab = top_tab_for(handle_in_x).expect("top fade-in grab tab");
    let fade_out_tab = top_tab_for(handle_out_x).expect("top fade-out grab tab");
    assert!(
        (fade_in_tab.width() - fade_in_tab.height()).abs() <= 0.001,
        "top fade-in tab should stay square"
    );
    assert!(
        (fade_out_tab.width() - fade_out_tab.height()).abs() <= 0.001,
        "top fade-out tab should stay square"
    );
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_bottom_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_mute_start_milli = Some(150);
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_mute_end_milli = Some(860);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let handle_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.15);
    let handle_out_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.86);

    let has_in_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.max.y == edit_rect.max.y
                    && rect.rect.height() < edit_rect.height()
                    && rect.rect.min.x < edit_rect.min.x
        )
    });
    let has_out_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.max.y == edit_rect.max.y
                    && rect.rect.height() < edit_rect.height()
                    && rect.rect.max.x > edit_rect.max.x
        )
    });
    assert!(has_in_tab, "expected bottom grab tab for fade-in handle");
    assert!(has_out_tab, "expected bottom grab tab for fade-out handle");
}

#[test]
fn waveform_motion_overlay_draws_square_edit_fade_bottom_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(495, 505);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(497);
    model.waveform.edit_fade_in_mute_start_milli = Some(490);
    model.waveform.edit_fade_out_start_milli = Some(503);
    model.waveform.edit_fade_out_mute_end_milli = Some(510);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let handle_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.49);
    let handle_out_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.51);

    let bottom_tab_for = |handle_x: f32| {
        frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect)
                    if rect.rect.min.x <= handle_x
                        && rect.rect.max.x >= handle_x
                        && rect.rect.max.y == edit_rect.max.y
                        && rect.rect.height() < edit_rect.height() =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .max_by(|left, right| {
                (left.width() * left.height())
                    .partial_cmp(&(right.width() * right.height()))
                    .expect("tab area should be comparable")
            })
    };

    let fade_in_tab = bottom_tab_for(handle_in_x).expect("bottom fade-in grab tab");
    let fade_out_tab = bottom_tab_for(handle_out_x).expect("bottom fade-out grab tab");
    assert!(
        (fade_in_tab.width() - fade_in_tab.height()).abs() <= 0.001,
        "bottom fade-in tab should stay square"
    );
    assert!(
        (fade_out_tab.width() - fade_out_tab.height()).abs() <= 0.001,
        "bottom fade-out tab should stay square"
    );
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_curve_trace() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_mute_start_milli = Some(150);
    model.waveform.edit_fade_in_curve_milli = Some(800);
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_mute_end_milli = Some(860);
    model.waveform.edit_fade_out_curve_milli = Some(250);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let fade_in_right = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.32);
    let fade_out_left = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.69);

    let has_left_curve_trace = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.width() <= 4.0
                    && rect.rect.height() <= 4.0
                    && rect.rect.max.x <= fade_in_right
                    && rect.rect.min.x < edit_rect.min.x
                    && rect.rect.min.x >= layout.waveform_plot.min.x
                    && rect.rect.min.y > edit_rect.min.y
                    && rect.rect.max.y < edit_rect.max.y
        )
    });
    let has_right_curve_trace = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.width() <= 4.0
                    && rect.rect.height() <= 4.0
                    && rect.rect.min.x >= fade_out_left
                    && rect.rect.max.x > edit_rect.max.x
                    && rect.rect.max.x <= layout.waveform_plot.max.x
                    && rect.rect.min.y > edit_rect.min.y
                    && rect.rect.max.y < edit_rect.max.y
        )
    });
    assert!(
        has_left_curve_trace,
        "expected fade-in curve markers past the selection start"
    );
    assert!(
        has_right_curve_trace,
        "expected fade-out curve markers past the selection end"
    );
}
