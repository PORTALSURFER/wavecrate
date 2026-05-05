use super::*;
use crate::compat_app_contract::FolderPaneIdModel;
#[test]
fn folder_create_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::create_draft(
            1,
            String::from("new folder"),
            String::from("New folder name"),
            Some(String::from("Folder already exists")),
            true,
        ),
    );
    let input_rect = state
        .folder_create_input_rect(&layout, &model)
        .expect("draft input should render");
    let text_rect = state
        .folder_create_text_rect(&layout, &model)
        .expect("draft text rect should render");
    state.set_folder_create_editor_state(Some(TextFieldVisualState {
        text: String::from("new folder"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == input_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x, text_rect.min.y),
                        Point::new(text_rect.min.x + 12.0, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x + 18.0, text_rect.min.y),
                        Point::new(text_rect.min.x + 18.0 + caret_width, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.text_runs.iter().any(|run| run.text == "new folder"));
}

#[test]
fn folder_rename_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::rename_draft(
            1,
            String::from("drums"),
            String::from("Folder name"),
            Some(String::from("Folder already exists")),
            true,
        ),
    );
    let input_rect = state
        .folder_create_input_rect(&layout, &model)
        .expect("rename draft input should render");
    let text_rect = state
        .folder_create_text_rect(&layout, &model)
        .expect("rename draft text rect should render");
    state.set_folder_create_editor_state(Some(TextFieldVisualState {
        text: String::from("drums"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == input_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x, text_rect.min.y),
                        Point::new(text_rect.min.x + 12.0, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x + 18.0, text_rect.min.y),
                        Point::new(text_rect.min.x + 18.0 + caret_width, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.text_runs.iter().any(|run| run.text == "drums"));
}

#[test]
fn source_divider_remains_above_tree_rows_in_cramped_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    let divider = compute_source_section_divider_rect(
        sections.source_rows(FolderPaneIdModel::Upper),
        sections.folder_header(FolderPaneIdModel::Upper),
        style.sizing,
    )
    .expect("divider should exist");
    assert_rect_inside(layout.sidebar_rows, divider);
    assert!(divider.max.y <= sections.tree_rows(FolderPaneIdModel::Upper).min.y);
    assert!(divider.min.y >= sections.source_rows(FolderPaneIdModel::Upper).min.y);
}

#[test]
fn recovery_badge_compacts_label_when_header_is_narrow() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(72.0, style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        false,
        153,
        true,
        true,
        false,
        true,
    );
    let badge = header_layout.badge.expect("badge should still render");
    assert_rect_inside(header_rect, badge.rect);
    assert!(badge.label.chars().count() <= 3);
    assert!(!badge.active);
}

#[test]
fn folder_header_text_width_yields_no_overlap_with_recovery_badge() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        true,
        0,
        true,
        true,
        false,
        true,
    );
    let badge = header_layout
        .badge
        .expect("badge should render for active recovery");
    assert!(header_layout.title_row.max.x <= badge.rect.min.x);
    if let Some(metadata_row) = header_layout.metadata_row {
        assert!(metadata_row.max.x <= badge.rect.min.x);
    }
}

#[test]
fn source_action_buttons_stay_inside_sidebar_footer() {
    let model = populated_sidebar_model();
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let buttons = source_action_buttons(&layout, &style, &model);
        assert!(!buttons.is_empty());
        for button in &buttons {
            assert_rect_inside(layout.sidebar_footer, button.rect);
        }
        for pair in buttons.windows(2) {
            assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
        }
    }
}

#[test]
fn selected_source_row_uses_mint_label_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(
        SourceRowModel::new("selected source", String::new(), false, false)
            .with_pane_assignment(true, false),
    );

    let frame = state.build_frame(&layout, &model);
    let selected_label = frame
        .text_runs
        .iter()
        .find(|run| run.text == "selected source")
        .expect("selected source label should render");

    assert_eq!(
        selected_label.color,
        StyleTokens::for_viewport_width(1280.0).accent_mint
    );
}

#[test]
fn recovery_badge_renders_idle_count_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.sources.recovery.entry_count = 153;
    model.sources.upper_folder_pane.recovery = model.sources.recovery.clone();

    let frame = state.build_frame(&layout, &model);
    let badge_label = frame
        .text_runs
        .iter()
        .find(|run| run.text == "153 entries")
        .expect("idle recovery badge label should render");

    assert_eq!(badge_label.color, style.text_primary);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { color, .. })
                if *color == style.chrome.source_recovery_badge_idle
        )
    }));
}
