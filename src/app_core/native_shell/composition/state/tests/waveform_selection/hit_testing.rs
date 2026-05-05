use super::*;
#[test]
fn waveform_motion_overlay_draws_edit_selection_shift_handle() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(240, 640);
    model.waveform.edit_selection_milli = Some(selection);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("edit selection rect");
    let handle_probe_x = selection_rect.min.x + (selection_rect.width() * 0.5);
    let handle_probe_y = selection_rect.max.y - 3.0;

    let has_shift_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_probe_x
                    && rect.rect.max.x >= handle_probe_x
                    && rect.rect.min.y <= handle_probe_y
                    && rect.rect.max.y >= handle_probe_y
                    && rect.rect.width() < selection_rect.width()
                    && rect.rect.height() < selection_rect.height()
        )
    });

    assert!(
        has_shift_handle,
        "expected edit-selection shift handle primitive"
    );
}

#[test]
fn waveform_motion_overlay_highlights_hovered_selection_resize_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
    state.hovered_waveform_resize_edge = Some(WaveformResizeHoverEdge::SelectionStart);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("selection rect");
    let edge_x = selection_rect.min.x;
    let center_y = selection_rect.min.y + (selection_rect.height() * 0.5);

    let has_edge_highlight = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= edge_x
                    && rect.rect.max.x >= edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    assert!(
        has_edge_highlight,
        "expected hovered selection edge highlight"
    );
}

#[test]
fn waveform_motion_overlay_uses_nano_view_bounds_for_selection_edges() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::from_micros(500_401, 500_402);
    model.waveform.selection_milli = Some(selection);
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 500;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 500_402;
    model.waveform.view_start_nanos = 500_400_250;
    model.waveform.view_end_nanos = 500_402_250;
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let precise_rect = compute_waveform_annotation_rects_with_nanos(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        model.waveform.view_start_nanos,
        model.waveform.view_end_nanos,
    )
    .selection
    .expect("precise selection rect");
    let quantized_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("quantized selection rect");

    assert!(
        frame.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(rect) if rect.rect == precise_rect)
        }),
        "motion overlay should render the nano-precise selection rect"
    );
    assert!(
        (precise_rect.min.x - quantized_rect.min.x).abs() > 10.0,
        "expected nano view bounds to materially change selection x; precise={} quantized={}",
        precise_rect.min.x,
        quantized_rect.min.x
    );
}

#[test]
fn waveform_resize_hit_testing_uses_rendered_snapped_selection_edges() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::from_micros(500_500, 500_900);
    model.waveform.selection_milli = Some(selection);
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 501;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 501_400;
    model.waveform.view_start_nanos = 500_400_000;
    model.waveform.view_end_nanos = 501_400_000;

    let selection_rect = compute_waveform_annotation_rects_with_nanos(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        model.waveform.view_start_nanos,
        model.waveform.view_end_nanos,
    )
    .selection
    .expect("selection rect");
    let probe_y = selection_rect.min.y + (selection_rect.height() * 0.5);

    assert_eq!(
        state.hovered_waveform_resize_edge_at_point(
            &layout,
            &model,
            Point::new(selection_rect.min.x, probe_y),
        ),
        Some(WaveformResizeHoverEdge::SelectionStart)
    );
    assert_eq!(
        state.hovered_waveform_resize_edge_at_point(
            &layout,
            &model,
            Point::new(selection_rect.max.x, probe_y),
        ),
        Some(WaveformResizeHoverEdge::SelectionEnd)
    );
}
