use super::*;
use crate::compat_app_contract::FolderPaneIdModel;

fn section_focus_color(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(
        style.bg_tertiary,
        style.accent_warning,
        (style.state_focus_pulse_blend * 0.12).clamp(0.06, 0.16),
    )
}

fn folder_browser_focus_rect(layout: &ShellLayout, style: &StyleTokens, model: &AppModel) -> Rect {
    let sections = sidebar_sections(layout, style, model);
    Rect::from_min_max(
        Point::new(
            sections
                .folder_header(FolderPaneIdModel::Upper)
                .min
                .x
                .min(sections.tree_rows(FolderPaneIdModel::Upper).min.x),
            sections
                .folder_header(FolderPaneIdModel::Upper)
                .min
                .y
                .min(sections.tree_rows(FolderPaneIdModel::Upper).min.y),
        ),
        Point::new(
            sections
                .folder_header(FolderPaneIdModel::Upper)
                .max
                .x
                .max(sections.tree_rows(FolderPaneIdModel::Upper).max.x),
            sections
                .folder_header(FolderPaneIdModel::Upper)
                .max
                .y
                .max(sections.tree_rows(FolderPaneIdModel::Upper).max.y),
        ),
    )
}

#[test]
fn waveform_focus_overlay_draws_waveform_card_surface() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
    state.sync_from_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let color = section_focus_color(&style);

    assert!(frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => rect.rect == layout.waveform_card && rect.color == color,
        _ => false,
    }));
}

#[test]
fn browser_focus_overlay_draws_browser_panel_surface() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.focus_context = crate::compat_app_contract::FocusContextModel::ContentList;
    state.sync_from_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let color = section_focus_color(&style);

    assert!(frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => rect.rect == layout.browser_panel && rect.color == color,
        _ => false,
    }));
}

#[test]
fn source_list_focus_overlay_draws_sidebar_source_band() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.focus_context = crate::compat_app_contract::FocusContextModel::NavigationList;
    state.sync_from_model(&model);

    let sections = sidebar_sections(&layout, &style, &model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let color = section_focus_color(&style);

    assert!(frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => {
            rect.rect == sections.source_rows(FolderPaneIdModel::Upper) && rect.color == color
        }
        _ => false,
    }));
}

#[test]
fn folder_browser_focus_overlay_draws_sidebar_folder_band() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.focus_context = crate::compat_app_contract::FocusContextModel::NavigationTree;
    state.sync_from_model(&model);

    let focus_rect = folder_browser_focus_rect(&layout, &style, &model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let color = section_focus_color(&style);

    assert!(frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => rect.rect == focus_rect && rect.color == color,
        _ => false,
    }));
}

#[test]
fn source_row_selected_fill_is_translucent_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(
        SourceRowModel::new("selected source", "detail", false, false)
            .with_pane_assignment(true, false),
    );

    let selected_row = *state
        .rendered_source_row_rects(&layout, &model)
        .first()
        .expect("source row should be rendered");
    let frame = state.build_frame(&layout, &model);

    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("selected source row should emit a fill rectangle");

    assert_eq!(
        row_color,
        translucent_overlay_color(
            style.bg_tertiary,
            style.grid_soft,
            style.state_selected_blend
        )
    );
    assert!(row_color.a < 255);
}

#[test]
fn browser_row_selected_fill_uses_stronger_neutral_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "selected row", 1, true, false));

    let selected_row = rendered_browser_rows(&layout, &model, &style)[0].rect;
    state.sync_from_model(&model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("selected browser row should emit a fill rectangle");

    assert_eq!(row_color, selected_browser_row_fill(&style));
}

#[test]
fn browser_row_selected_state_highlights_index_column() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "selected row", 1, true, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "plain row", 1, false, false));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let selected_index_rect = rendered[0].text_layout.columns.index;
    let plain_index_rect = rendered[1].text_layout.columns.index;
    state.sync_from_model(&model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == selected_index_rect
                    && *color == selected_browser_index_fill(&style)
        )
    }));
    assert!(!frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == plain_index_rect
                    && *color == selected_browser_index_fill(&style)
        )
    }));
}

#[test]
fn browser_row_locked_selected_fill_matches_standard_selection_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, true, false).with_locked(true));

    let selected_row = rendered_browser_rows(&layout, &model, &style)[0].rect;
    state.sync_from_model(&model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("locked browser row should emit a fill rectangle");

    assert_eq!(row_color, selected_browser_row_fill(&style));
}

#[test]
fn browser_row_locked_selected_state_draws_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, true, false).with_locked(true));

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("locked marker");
    state.sync_from_model(&model);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.accent_mint
        )
    }));
}

#[test]
fn browser_row_text_revision_changes_when_locked_state_changes() {
    let unlocked = [BrowserRowModel::new(0, "row", 1, false, false)];
    let locked = [BrowserRowModel::new(0, "row", 1, false, false).with_locked(true)];

    assert_ne!(
        browser_row_text_revision(&unlocked),
        browser_row_text_revision(&locked)
    );
}

#[test]
fn browser_row_text_revision_changes_when_similarity_strength_changes() {
    let weaker =
        [BrowserRowModel::new(0, "row", 1, false, false).with_similarity_display_strength(0.2)];
    let stronger =
        [BrowserRowModel::new(0, "row", 1, false, false).with_similarity_display_strength(0.9)];

    assert_ne!(
        browser_row_text_revision(&weaker),
        browser_row_text_revision(&stronger)
    );
}

#[test]
fn browser_row_selected_state_does_not_draw_mint_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "selected row", 1, true, false));

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let mint_border = blend_color(
        style.accent_mint,
        style.text_primary,
        style.state_selected_blend,
    );
    let has_mint_top_border =
        state
            .build_frame(&layout, &model)
            .primitives
            .iter()
            .any(|primitive| match primitive {
                Primitive::Rect(rect) => {
                    rect.color == mint_border
                        && rect.rect.min.x == border_rect.min.x
                        && rect.rect.max.x == border_rect.max.x
                        && rect.rect.min.y == border_rect.min.y
                        && rect.rect.max.y == border_rect.min.y + stroke
                }
                _ => false,
            });

    assert!(
        !has_mint_top_border,
        "selected browser rows should rely on fill instead of mint borders"
    );
}

#[test]
fn browser_row_focused_state_draws_bottom_focus_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, false, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let has_focus_bottom_border = frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => {
            rect.color == focus_border
                && rect.rect.min.x == border_rect.min.x
                && rect.rect.max.x == border_rect.max.x
                && rect.rect.min.y == border_rect.max.y - stroke
                && rect.rect.max.y == border_rect.max.y
        }
        _ => false,
    });

    assert!(
        has_focus_bottom_border,
        "focused browser rows should render a full border highlight"
    );
}

#[test]
fn browser_row_focused_state_draws_left_focus_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, false, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let has_focus_left_border = frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => {
            rect.color == focus_border
                && rect.rect.min.x == border_rect.min.x
                && rect.rect.max.x == border_rect.min.x + stroke
                && rect.rect.min.y == border_rect.min.y
                && rect.rect.max.y == border_rect.max.y
        }
        _ => false,
    });

    assert!(
        has_focus_left_border,
        "focused browser rows should keep their left focus border highlight"
    );
}

#[test]
fn browser_row_locked_focused_state_draws_offset_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked focused row", 1, false, true).with_locked(true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, stroke).expect("locked marker");
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.accent_mint
        )
    }));
    assert!(
        marker_rect.min.x >= border_rect.min.x + stroke,
        "locked marker should sit to the right of the focus border"
    );
}

#[test]
fn browser_row_selected_focused_state_keeps_index_highlight() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(BrowserRowModel::new(
        0,
        "focused selected row",
        1,
        true,
        true,
    ));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row.text_layout.columns.index
                    && *color == selected_browser_index_fill(&style)
        )
    }));
}

#[test]
fn similarity_anchor_selected_focused_state_uses_blue_index_highlight() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.similarity_filtered = true;
    model.browser.rows.push(BrowserRowModel::new(
        0,
        "focused selected anchor",
        1,
        true,
        true,
    ));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "match row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row.text_layout.columns.index
                    && *color == similarity_anchor_browser_index_fill(&style)
        )
    }));
}
