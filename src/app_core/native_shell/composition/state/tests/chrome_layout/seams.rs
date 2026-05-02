use super::super::*;

#[test]
fn waveform_scrollbar_lane_stays_separate_from_waveform_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(
        layout.waveform_plot.min.x,
        layout.waveform_card.min.x + 10.0
    );
    assert_eq!(
        layout.waveform_plot.max.x,
        layout.waveform_card.max.x - 10.0
    );
    assert_eq!(layout.waveform_plot.min.y, layout.waveform_header.max.y);
    assert_eq!(
        layout.waveform_scrollbar_lane.min.x,
        layout.waveform_card.min.x + 10.0
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.max.x,
        layout.waveform_card.max.x - 10.0
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.max.y,
        layout.waveform_card.max.y
    );
    assert_eq!(
        layout.waveform_plot.max.y,
        layout.waveform_scrollbar_lane.min.y
    );
    assert!(layout.waveform_scrollbar_lane.height() >= 12.0);
}

#[test]
fn touching_major_panels_render_single_seam_borders() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);
    let stroke = style.sizing.border_width.max(1.0);
    let top_body_seam = Rect::from_min_max(
        Point::new(layout.root.rect.min.x, layout.top_bar.max.y - stroke),
        Point::new(layout.root.rect.max.x, layout.top_bar.max.y),
    );
    let sidebar_content_seam = Rect::from_min_max(
        Point::new(layout.sidebar.max.x - stroke, layout.top_bar.max.y),
        Point::new(layout.sidebar.max.x, layout.status_bar.min.y),
    );
    let top_body_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == top_body_seam && *color == style.border
            )
        })
        .count();
    let sidebar_content_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == sidebar_content_seam && *color == style.border
            )
        })
        .count();
    let status_bar_bottom_seam = Rect::from_min_max(
        Point::new(layout.status_bar.min.x, layout.status_bar.max.y - stroke),
        Point::new(layout.status_bar.max.x, layout.status_bar.max.y),
    );
    let status_bar_bottom_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == status_bar_bottom_seam && *color == style.border
            )
        })
        .count();
    assert_eq!(top_body_matches, 1);
    assert_eq!(sidebar_content_matches, 1);
    assert_eq!(status_bar_bottom_matches, 0);
}

#[test]
fn chrome_motion_status_overlay_preserves_status_bar_border_lines() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.status.right = String::from("2/3");
    let motion = NativeMotionModel::from_app_model(&model);
    let overlay_segment = layout.status_right_segment;
    let overlay_rect = status_motion_overlay_rect(overlay_segment, style.sizing.border_width);

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    assert!(
        frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == overlay_rect && *color == style.surface_raised
            )
        }),
        "status motion overlay should repaint only the inset text background"
    );
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == layout.status_right_segment && *color == style.surface_raised
            )
        }),
        "status motion overlay should not cover the full status segment and erase border lines"
    );
}
