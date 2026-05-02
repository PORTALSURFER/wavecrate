use super::*;

#[test]
fn hovered_sections_do_not_emit_panel_fill_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    for hovered in [
        ShellNodeKind::TopBar,
        ShellNodeKind::Sidebar,
        ShellNodeKind::WaveformCard,
    ] {
        let mut frame = NativeViewFrame::default();
        state.hovered = Some(hovered);
        state.build_state_overlay_into(&layout, &style, &model, &mut frame);
        assert!(
            frame.primitives.iter().all(|primitive| {
                !matches!(
                    primitive,
                    Primitive::Rect(rect)
                        if rect.rect == layout.top_bar
                            || rect.rect == layout.sidebar
                            || rect.rect == layout.waveform_card
                )
            }),
            "hovered section should not emit a panel-sized fill overlay"
        );
    }
}

#[test]
fn browser_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "hover", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "hover-2", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered_rows = rendered_browser_rows(&layout, &model, &style);
    let hover_row = rendered_rows[0].rect;
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = browser_row_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("hovered browser row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
}

#[test]
fn folder_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();

    let rendered_rows = rendered_folder_row_rects(&layout, &style, &model);
    let hover_row = rendered_rows[0];
    let hover_visual_rect = folder_row_visual_rect(hover_row, style.sizing);
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::Sidebar));
    assert!(fingerprint.hovered_folder_row_index.is_some());

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = subtle_item_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_visual_rect => Some(rect.color),
            _ => None,
        })
        .expect("hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(rect) if rect.rect == hover_row && rect.color == expected_hover
        )
    }));
}

#[test]
fn folder_row_drag_hovered_overlay_uses_drag_target_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = populated_sidebar_model();
    model.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("kick"),
        target_label: String::from("Folder: Drums"),
        valid_target: true,
        pointer_x: Some(32),
        pointer_y: Some(32),
    };

    let rendered_rows = rendered_folder_row_rects(&layout, &style, &model);
    let hover_row = rendered_rows[0];
    let hover_visual_rect = folder_row_visual_rect(hover_row, style.sizing);
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = folder_drag_hover_fill(&style, true);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_visual_rect => Some(rect.color),
            _ => None,
        })
        .expect("drag-hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(rect) if rect.rect == hover_row && rect.color == expected_hover
        )
    }));
}

#[test]
fn folder_panel_background_drag_does_not_emit_row_highlight() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = populated_sidebar_model();
    model.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("kick"),
        target_label: String::from("Folder panel"),
        valid_target: true,
        pointer_x: Some(32),
        pointer_y: Some(32),
    };

    let background_point = sidebar_background_point(&mut state, &layout, &model);
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, background_point),
        CursorMoveEffect::None
    );
    assert_eq!(
        state.state_overlay_fingerprint().hovered,
        Some(ShellNodeKind::Sidebar)
    );
    assert_eq!(
        state.state_overlay_fingerprint().hovered_folder_row_index,
        None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let folder_row_rects = rendered_folder_row_rects(&layout, &style, &model);
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect)
                    if folder_row_rects.contains(&rect.rect)
                        && rect.color == folder_drag_hover_fill(&style, true)
            )
        }),
        "dragging over empty folder-panel background should not emit a folder-row highlight"
    );
}

#[test]
fn clearing_browser_row_hover_removes_unrelated_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(40, 18);
    let hovered_row = rendered_browser_rows(&layout, &model, &style)
        .into_iter()
        .find(|row| row.visible_row == 12)
        .map(|row| row.rect)
        .expect("hover target row should render");
    let hover_point = Point::new(
        hovered_row.min.x + 6.0,
        (hovered_row.min.y + hovered_row.max.y) * 0.5,
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, hover_point),
        CursorMoveEffect::None
    );
    assert_eq!(
        state
            .state_overlay_fingerprint()
            .hovered_browser_visible_row,
        Some(12)
    );

    state.clear_browser_row_hover();
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert_eq!(
        state
            .state_overlay_fingerprint()
            .hovered_browser_visible_row,
        None
    );
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect)
                    if rect.rect == hovered_row && rect.color == browser_row_hover_fill(&style)
            )
        }),
        "cleared browser row hover should remove the row-hover fill"
    );
}

fn sidebar_background_point(
    state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if state.folder_panel_contains_point(layout, model, point)
                && state.folder_row_at_point(layout, model, point).is_none()
                && state.source_row_at_point(layout, model, point).is_none()
            {
                return point;
            }
        }
    }
    panic!("expected hittable folder-panel background point");
}
