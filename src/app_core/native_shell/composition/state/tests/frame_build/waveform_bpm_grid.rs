use super::*;

#[test]
fn top_bar_omits_status_indicator_dot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;

    let frame = state.build_frame(&layout, &model);

    assert!(
        !frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle(_))),
        "top-right status dot should not be rendered"
    );

    let style = style_for_layout(&layout);
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    assert!(
        !overlay
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle(_))),
        "motion overlay should not reintroduce the top-right status dot"
    );
}

#[test]
fn waveform_bpm_grid_lines_render_from_sample_origin_when_no_selection_exists() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.view_start_micros = 125_000;
    model.waveform.view_end_micros = 875_000;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = true;
    let beat_line_color = blend_color(style.grid_soft, style.text_muted, 0.32);

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [0.125_f32, 0.25, 0.375, 0.625, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.5)];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);

    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.y == layout.waveform_plot.min.y
                    && rect.rect.max.y == layout.waveform_plot.max.y
                    && rect.color == beat_line_color
        )),
        "waveform beat grid should render when BPM snap is enabled"
    );
}

#[test]
fn waveform_bpm_grid_lines_reuse_last_selection_origin_after_clear() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.view_start_micros = 125_000;
    model.waveform.view_end_micros = 875_000;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = true;
    model.waveform.selection_milli = Some(crate::compat_app_contract::NormalizedRangeModel::new(
        125, 375,
    ));

    let selected_frame = state.build_frame(&layout, &model);
    let selected_positions = waveform_bpm_grid_positions(&selected_frame, &layout, &style);
    let expected_selected_soft_xs = [0.25_f32, 0.375, 0.5, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_selected_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.125),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.625),
    ];

    assert_eq!(selected_positions.0, expected_selected_soft_xs);
    assert_eq!(selected_positions.1, expected_selected_strong_xs);

    model.waveform.selection_milli = None;
    let cleared_frame = state.build_frame(&layout, &model);
    let cleared_positions = waveform_bpm_grid_positions(&cleared_frame, &layout, &style);
    let expected_cleared_soft_xs = [0.25_f32, 0.375, 0.5, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_cleared_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.125),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.625),
    ];

    assert_eq!(cleared_positions.0, expected_cleared_soft_xs);
    assert_eq!(cleared_positions.1, expected_cleared_strong_xs);
}

#[test]
fn waveform_bpm_grid_lines_prefer_projected_origin_when_present() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.view_start_micros = 125_000;
    model.waveform.view_end_micros = 875_000;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform.bpm_grid_origin_micros = 250_000;
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = true;

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [0.375_f32, 0.5, 0.625, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.25),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.75),
    ];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);
}

#[test]
fn waveform_bpm_grid_lines_follow_micro_precision_viewport_when_zoomed() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 501;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 501_400;
    model.waveform.beat_step_micros = Some(200);
    model.waveform.bpm_grid_origin_micros = 500_500;
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = true;

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [500_700_u32, 500_900, 501_100]
        .into_iter()
        .map(|beat| beat_grid_x_micros(layout.waveform_plot, 500_400, 501_400, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![
        beat_grid_x_micros(layout.waveform_plot, 500_400, 501_400, 500_500),
        beat_grid_x_micros(layout.waveform_plot, 500_400, 501_400, 501_300),
    ];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);
}

#[test]
fn waveform_bpm_grid_lines_align_with_snapped_selection_edges_in_relative_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 501;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 501_400;
    model.waveform.view_start_nanos = 500_400_000;
    model.waveform.view_end_nanos = 501_400_000;
    model.waveform.beat_step_micros = Some(200);
    model.waveform.bpm_grid_origin_micros = 500_500;
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = true;
    model.waveform.selection_milli = Some(NormalizedRangeModel::from_micros(500_500, 500_900));

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    let selection_rect = compute_waveform_annotation_rects_with_nanos(
        layout.waveform_plot,
        style.sizing.border_width,
        model.waveform.selection_milli,
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        model.waveform.view_start_nanos,
        model.waveform.view_end_nanos,
    )
    .selection
    .expect("selection rect");
    let all_grid_xs = soft_xs
        .iter()
        .chain(strong_xs.iter())
        .copied()
        .collect::<Vec<_>>();

    assert!(
        overlay.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(rect) if rect.rect == selection_rect)
        }),
        "motion overlay should render the snapped playback selection rect"
    );
    assert!(all_grid_xs.contains(&selection_rect.min.x));
    assert!(all_grid_xs.contains(&selection_rect.max.x));
}

#[test]
fn waveform_bpm_grid_lines_align_with_snapped_selection_edges_in_global_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 501;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 501_400;
    model.waveform.view_start_nanos = 500_400_000;
    model.waveform.view_end_nanos = 501_400_000;
    model.waveform.beat_step_micros = Some(200);
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = false;
    model.waveform.selection_milli = Some(NormalizedRangeModel::from_micros(500_600, 501_000));

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    let selection_rect = compute_waveform_annotation_rects_with_nanos(
        layout.waveform_plot,
        style.sizing.border_width,
        model.waveform.selection_milli,
        None,
        None,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        model.waveform.view_start_nanos,
        model.waveform.view_end_nanos,
    )
    .selection
    .expect("selection rect");
    let all_grid_xs = soft_xs
        .iter()
        .chain(strong_xs.iter())
        .copied()
        .collect::<Vec<_>>();

    assert!(
        overlay.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(rect) if rect.rect == selection_rect)
        }),
        "motion overlay should render the snapped playback selection rect"
    );
    assert!(all_grid_xs.contains(&selection_rect.min.x));
    assert!(all_grid_xs.contains(&selection_rect.max.x));
}

#[test]
fn waveform_bpm_grid_lines_ignore_selection_origin_in_global_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.view_start_micros = 125_000;
    model.waveform.view_end_micros = 875_000;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform_chrome.relative_bpm_grid_enabled = false;
    model.waveform.selection_milli = Some(crate::compat_app_contract::NormalizedRangeModel::new(
        125, 375,
    ));
    model.waveform.bpm_grid_origin_micros = 250_000;

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [0.125_f32, 0.25, 0.375, 0.625, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.5)];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);
}

fn beat_grid_x(waveform_plot: Rect, view_start: f32, view_end: f32, beat: f32) -> f32 {
    let ratio = (beat - view_start) / (view_end - view_start);
    (waveform_plot.min.x + (waveform_plot.width() * ratio)).round()
}

fn beat_grid_x_micros(
    waveform_plot: Rect,
    view_start_micros: u32,
    view_end_micros: u32,
    beat_micros: u32,
) -> f32 {
    let view_width = view_end_micros.saturating_sub(view_start_micros).max(1) as f32;
    let ratio = beat_micros.saturating_sub(view_start_micros) as f32 / view_width;
    (waveform_plot.min.x + (waveform_plot.width() * ratio)).round()
}

fn waveform_bpm_grid_positions(
    frame: &NativeViewFrame,
    layout: &ShellLayout,
    style: &StyleTokens,
) -> (Vec<f32>, Vec<f32>) {
    let beat_line_color = blend_color(style.grid_soft, style.text_muted, 0.32);
    let mut soft_xs = Vec::new();
    let mut strong_xs = Vec::new();
    for primitive in &frame.primitives {
        let Primitive::Rect(rect) = primitive else {
            continue;
        };
        if rect.rect.min.y != layout.waveform_plot.min.y
            || rect.rect.max.y != layout.waveform_plot.max.y
        {
            continue;
        }
        if rect.color == beat_line_color {
            soft_xs.push(rect.rect.min.x);
        } else if rect.color == style.grid_strong {
            strong_xs.push(rect.rect.min.x);
        }
    }
    (soft_xs, strong_xs)
}
