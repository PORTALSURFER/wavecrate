use super::*;

#[test]
fn waveform_motion_overlay_omits_edit_resize_handles_until_hovered() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
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
    let left_edge_x = edit_rect.min.x;
    let right_edge_x = edit_rect.max.x;
    let edit_center_y = edit_rect.min.y + (edit_rect.height() * 0.5);

    let has_left_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= left_edge_x
                    && rect.rect.max.x >= left_edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - edit_center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    let has_right_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= right_edge_x
                    && rect.rect.max.x >= right_edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - edit_center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(
        !has_left_handle,
        "edit edges should not draw standalone handles"
    );
    assert!(
        !has_right_handle,
        "edit edges should not draw standalone handles"
    );
}

#[test]
fn waveform_motion_overlay_highlights_hovered_edit_resize_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    state.hovered_waveform_resize_edge = Some(WaveformResizeHoverEdge::EditSelectionEnd);
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
    let edge_x = edit_rect.max.x;
    let center_y = edit_rect.min.y + (edit_rect.height() * 0.5);

    let has_edge_highlight = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= edge_x
                    && rect.rect.max.x >= edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(has_edge_highlight, "expected hovered edit edge highlight");
}

#[test]
fn waveform_motion_overlay_draws_loop_range_bar_when_loop_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let play_selection = NormalizedRangeModel::new(260, 620);
    model.waveform.selection_milli = Some(play_selection);
    model.waveform.loop_enabled = true;
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(play_selection),
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .selection
    .expect("play selection rect");
    let bar_height = 3.0f32
        .max(style.sizing.border_width)
        .min(selection_rect.height().max(1.0));
    let top = Rect::from_min_max(
        selection_rect.min,
        Point::new(
            selection_rect.max.x,
            (selection_rect.min.y + bar_height).min(selection_rect.max.y),
        ),
    );
    let bottom = Rect::from_min_max(
        Point::new(
            selection_rect.min.x,
            (selection_rect.max.y - bar_height).max(selection_rect.min.y),
        ),
        selection_rect.max,
    );
    let top_color = translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.42);
    let bottom_color = translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.32);

    let has_top = frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == top && rect.color == top_color)
    });
    let has_bottom = frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == bottom && rect.color == bottom_color)
    });
    assert!(has_top, "expected top loop-range bar fill");
    assert!(has_bottom, "expected bottom loop-range bar fill");
}

#[test]
fn waveform_motion_overlay_hides_edit_fade_bottom_grab_tabs_without_fades() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
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
    let handle_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let handle_out_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8);

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
    assert!(
        !has_in_tab,
        "bottom fade-in tab should stay hidden without a fade"
    );
    assert!(
        !has_out_tab,
        "bottom fade-out tab should stay hidden without a fade"
    );
}
