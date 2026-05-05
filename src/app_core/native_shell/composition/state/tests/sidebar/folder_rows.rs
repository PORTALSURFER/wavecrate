use super::*;

#[test]
fn folder_row_label_rect_indents_children_beyond_root() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Drums", String::new(), 1, false, true, false, true, true),
    );

    let rows = rendered_folder_row_rects(&layout, &style, &model);
    let root_layout = compute_sidebar_folder_row_layout(
        rows[0],
        style.sizing,
        compute_sidebar_folder_row_depth_indent(rows[0], style.sizing, 0),
    );
    let child_layout = compute_sidebar_folder_row_layout(
        rows[1],
        style.sizing,
        compute_sidebar_folder_row_depth_indent(rows[1], style.sizing, 1),
    );

    assert!(child_layout.label_rect.min.x > root_layout.label_rect.min.x);
    assert!(child_layout.disclosure_rect.min.x > root_layout.disclosure_rect.min.x);
}

#[test]
fn tree_rows_render_plain_labels_without_fallback_glyph_prefixes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Drums", String::new(), 1, false, true, false, true, true),
    );

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(frame.text_runs.iter().any(|run| run.text == "Drums"));
    assert!(frame.text_runs.iter().all(|run| !matches!(
        run.text.as_str(),
        "• Root" | "▶ Drums" | "▼ Drums" | "· Drums"
    )));
}

#[test]
fn disclosure_gutter_hit_target_is_reserved_only_for_expandable_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Drums", String::new(), 1, false, true, false, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "OneShots",
            String::new(),
            2,
            false,
            false,
            false,
            false,
            false,
        ),
    );

    let mut state = NativeShellState::new();
    assert!(
        state
            .folder_row_disclosure_rect(&layout, &model, 1)
            .expect("expandable folder should reserve a gutter")
            .width()
            > 1.0
    );

    let leaf_rect = state
        .folder_row_disclosure_rect(&layout, &model, 2)
        .expect("leaf rows still compute gutter geometry");
    let leaf_point = Point::new(
        (leaf_rect.min.x + leaf_rect.max.x) * 0.5,
        (leaf_rect.min.y + leaf_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, leaf_point),
        None
    );
}

#[test]
fn tree_rows_use_single_pixel_shared_separator() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "folder_a",
            String::new(),
            0,
            false,
            false,
            false,
            true,
            true,
        ),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "folder_b",
            String::new(),
            0,
            false,
            false,
            false,
            true,
            true,
        ),
    );

    let tree_rows = rendered_folder_row_rects(&layout, &style, &model);
    assert!(tree_rows.len() >= 2, "expected at least two folder rows");
    let first_visual_rect = folder_row_visual_rect(tree_rows[0], style.sizing);
    let shared_boundary_y = tree_rows[1].min.y;
    let stroke = style.sizing.border_width.max(1.0);

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    let top_separator_count = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *color == style.border
                        && rect.min.x == first_visual_rect.min.x
                        && rect.max.x == first_visual_rect.max.x
                        && rect.min.y == shared_boundary_y
                        && rect.max.y == shared_boundary_y + stroke
            )
        })
        .count();
    let lower_stacked_separator_count = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *color == style.border
                        && rect.min.x == first_visual_rect.min.x
                        && rect.max.x == first_visual_rect.max.x
                        && rect.min.y == shared_boundary_y - stroke
                        && rect.max.y == shared_boundary_y
            )
        })
        .count();

    assert_eq!(
        top_separator_count, 1,
        "expected one shared folder-row separator"
    );
    assert_eq!(
        lower_stacked_separator_count, 0,
        "folder rows should not stack a second border under the shared separator"
    );
}

#[test]
fn plain_folder_row_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "folder_plain",
            String::new(),
            0,
            false,
            false,
            false,
            true,
            true,
        ),
    );

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(visual_rect.min.x > row_rect.min.x);
    assert!(visual_rect.max.x < row_rect.max.x);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == style.surface_base
        )
    }));
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == style.surface_base
        )
    }));
}

#[test]
fn selected_folder_row_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "folder_selected",
            String::new(),
            0,
            true,
            false,
            false,
            true,
            true,
        ),
    );

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let expected_fill = translucent_overlay_color(
        style.bg_tertiary,
        style.grid_soft,
        style.state_selected_blend,
    );
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == expected_fill
        )
    }));
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == expected_fill
        )
    }));
}

#[test]
fn focused_folder_overlay_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.focus_context = crate::compat_app_contract::FocusContextModel::NavigationTree;
    push_active_folder_row(
        &mut model,
        FolderRowModel::new(
            "folder_focused",
            String::new(),
            0,
            false,
            true,
            false,
            true,
            true,
        ),
    );

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let expected_fill = translucent_overlay_color(
        style.bg_tertiary,
        style.grid_strong,
        style.state_focus_pulse_blend,
    );
    let mut state = NativeShellState::new();
    state.has_focus_emphasis = true;
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == expected_fill
        )
    }));
    assert!(overlay.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == expected_fill
        )
    }));
}
