use super::*;

#[test]
fn waveform_motion_overlay_draws_distinct_play_and_edit_selection_marks() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let play_selection = NormalizedRangeModel::new(180, 420);
    let edit_selection = NormalizedRangeModel::new(560, 820);
    model.waveform.selection_milli = Some(play_selection);
    model.waveform.edit_selection_milli = Some(edit_selection);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let play_rect = compute_waveform_annotation_rects(
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

    let play_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == play_rect => Some(rect.color),
            _ => None,
        })
        .expect("play selection fill");
    let edit_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == edit_rect => Some(rect.color),
            _ => None,
        })
        .expect("edit selection fill");
    assert_eq!(
        play_fill,
        translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52)
    );
    assert_eq!(
        edit_fill,
        translucent_overlay_color(style.bg_secondary, style.highlight_blue, 0.5)
    );
    assert_ne!(play_fill, edit_fill);
}

#[test]
fn waveform_motion_overlay_flashes_selection_after_export_success_token() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
    model.waveform.selection_export_flash_nonce = 1;
    let motion = NativeMotionModel::from_app_model(&model);
    state.sync_from_motion_model(&motion);

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
    let flash_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selection_rect => Some(rect.color),
            _ => None,
        })
        .expect("selection flash fill");

    assert_eq!(
        flash_fill,
        translucent_overlay_color(style.surface_overlay, style.accent_warning, 0.78)
    );
}

#[test]
fn waveform_motion_overlay_flashes_selection_red_after_export_failure_token() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
    model.waveform.selection_export_failure_flash_nonce = 1;
    let motion = NativeMotionModel::from_app_model(&model);
    state.sync_from_motion_model(&motion);

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
    let flash_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selection_rect => Some(rect.color),
            _ => None,
        })
        .expect("selection flash fill");

    assert_eq!(
        flash_fill,
        translucent_overlay_color(style.surface_overlay, style.accent_trash, 0.78)
    );
}

#[test]
fn waveform_motion_overlay_flashes_edit_selection_after_apply_token() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(560, 820);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_selection_apply_flash_nonce = 1;
    let motion = NativeMotionModel::from_app_model(&model);
    state.sync_from_motion_model(&motion);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
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
    let flash_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selection_rect => Some(rect.color),
            _ => None,
        })
        .expect("edit selection flash fill");

    assert_eq!(
        flash_fill,
        translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.82)
    );
}

#[test]
fn waveform_motion_overlay_omits_selection_resize_handles_until_hovered() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
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
    let left_edge_x = selection_rect.min.x;
    let right_edge_x = selection_rect.max.x;
    let selection_center_y = selection_rect.min.y + (selection_rect.height() * 0.5);

    let has_left_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= left_edge_x
                    && rect.rect.max.x >= left_edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - selection_center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    let has_right_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= right_edge_x
                    && rect.rect.max.x >= right_edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - selection_center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    assert!(
        !has_left_handle,
        "selection edges should not draw standalone handles"
    );
    assert!(
        !has_right_handle,
        "selection edges should not draw standalone handles"
    );
}

#[test]
fn waveform_motion_overlay_draws_selection_drag_handle() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
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
    let export_handle_probe_x = selection_rect.max.x - 6.0;
    let bottom_handle_probe_x = selection_rect.min.x + (selection_rect.width() * 0.5);
    let handle_probe_y = selection_rect.max.y - 3.0;

    let has_export_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= export_handle_probe_x
                    && rect.rect.max.x >= export_handle_probe_x
                    && rect.rect.min.y <= handle_probe_y
                    && rect.rect.max.y >= handle_probe_y
                    && rect.rect.width() < selection_rect.width()
                    && rect.rect.height() < selection_rect.height()
        )
    });
    let has_shift_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= bottom_handle_probe_x
                    && rect.rect.max.x >= bottom_handle_probe_x
                    && rect.rect.min.y <= handle_probe_y
                    && rect.rect.max.y >= handle_probe_y
                    && rect.rect.width() < selection_rect.width()
                    && rect.rect.height() < selection_rect.height()
        )
    });

    assert!(
        has_export_handle,
        "expected playback-selection drag handle primitive"
    );
    assert!(
        has_shift_handle,
        "expected playback-selection shift handle primitive"
    );
}
