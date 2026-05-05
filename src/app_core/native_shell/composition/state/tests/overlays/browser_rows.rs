use super::*;

#[test]
fn stale_static_browser_rows_do_not_keep_old_focus_highlight_after_refocus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let old_model = browser_model_with_rows(40, 18);
    let new_model = browser_model_with_rows(40, 12);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );

    let old_row_rect = rendered_browser_rows(&layout, &old_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 18)
        .map(|row| row.rect)
        .expect("old focused row should render");
    let new_row_rect = rendered_browser_rows(&layout, &new_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 12)
        .map(|row| row.rect)
        .expect("new focused row should render");

    let mut segments = StaticFrameSegments::default();
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &old_model,
        None,
        StaticFrameSegment::BrowserRowsWindow,
        &mut segments,
    );

    let static_frame = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    assert!(
        static_frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect.min.y >= old_row_rect.min.y
                    && rect.rect.max.y <= old_row_rect.max.y
                    && rect.color == focus_border
            )
        }),
        "static browser rows should not own focused-row warning chrome"
    );

    state.sync_from_model(&new_model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &new_model, &mut overlay);

    let old_focus_rects = overlay
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.min.y >= old_row_rect.min.y
                    && rect.rect.max.y <= old_row_rect.max.y
                    && rect.color == focus_border
            }
            _ => false,
        })
        .count();
    let new_focus_rects = overlay
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.min.y >= new_row_rect.min.y
                    && rect.rect.max.y <= new_row_rect.max.y
                    && rect.color == focus_border
            }
            _ => false,
        })
        .count();

    assert_eq!(
        old_focus_rects, 0,
        "fresh overlay should not keep the old focused row highlighted"
    );
    let _ = new_focus_rects;
}

#[test]
fn stale_static_browser_rows_do_not_keep_old_selection_fill_after_refocus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut old_model = browser_model_with_rows(40, 18);
    let mut new_model = browser_model_with_rows(40, 12);
    if let Some(row) = old_model.browser.rows.get_mut(18) {
        row.selected = true;
    }
    if let Some(row) = new_model.browser.rows.get_mut(12) {
        row.selected = true;
    }
    let old_row_rect = rendered_browser_rows(&layout, &old_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 18)
        .map(|row| row.rect)
        .expect("old selected row should render");
    let selected_fill = selected_browser_row_fill(&style);

    let mut segments = StaticFrameSegments::default();
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &old_model,
        None,
        StaticFrameSegment::BrowserRowsWindow,
        &mut segments,
    );

    let static_frame = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    assert!(
        static_frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect == old_row_rect && rect.color == selected_fill
            )
        }),
        "static browser rows should not own selected-row fill"
    );

    state.sync_from_model(&new_model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &new_model, &mut overlay);

    assert!(
        overlay.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect == old_row_rect && rect.color == selected_fill
            )
        }),
        "fresh overlay should not keep the old selected row filled"
    );
}
