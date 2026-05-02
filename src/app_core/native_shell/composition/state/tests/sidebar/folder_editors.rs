use super::*;
#[test]
fn inline_folder_draft_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::create_draft(
            0,
            String::from("new folder"),
            String::from("New folder name"),
            None,
            true,
        ),
    );

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

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
fn waveform_bpm_input_focus_overlay_uses_active_input_chrome() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let motion = NativeMotionModel::from_app_model(&AppModel::default());
    let mut state = NativeShellState::new();
    state.set_waveform_bpm_editor_state(true, Some(String::from("128.0")), None);
    let bpm_rect = state
        .waveform_toolbar_button_rect(&layout, &AppModel::default(), "BPM Value")
        .expect("bpm value waveform toolbar widget should be present");

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == bpm_rect => Some(*color),
            _ => None,
        })
        .expect("active bpm input should emit a focus overlay fill");

    assert_eq!(
        overlay_color,
        waveform_bpm_input_focus_fill(&style, interaction_wave(0.0))
    );
}

#[test]
fn waveform_bpm_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let motion = NativeMotionModel::from_app_model(&AppModel::default());
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let bpm_rect = state
        .waveform_bpm_input_rect(&layout, &model)
        .expect("bpm field should render");
    let bpm_text = state
        .waveform_bpm_text_rect(&layout, &model)
        .expect("bpm text rect should render");
    state.set_waveform_bpm_editor_state(
        true,
        Some(String::from("128.0")),
        Some(TextFieldVisualState {
            text: String::from("128.0"),
            caret_offset: 22.0,
            selection_offsets: Some((0.0, 16.0)),
        }),
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == bpm_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(bpm_text.min.x, bpm_text.min.y),
                        Point::new(bpm_text.min.x + 16.0, bpm_text.max.y),
                    )
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(bpm_text.min.x + 22.0, bpm_text.min.y),
                        Point::new(bpm_text.min.x + 22.0 + caret_width, bpm_text.max.y),
                    )
        )
    }));
    assert!(frame.text_runs.iter().any(|run| run.text == "128.0"));
}

#[test]
fn folder_create_draft_row_ignores_disclosure_hit_testing() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
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
            None,
            true,
        ),
    );
    let mut state = NativeShellState::new();
    let disclosure = state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("draft row should still have layout geometry");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, point),
        None
    );
}

#[test]
fn folder_rename_draft_row_ignores_disclosure_hit_testing() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    push_active_folder_row(
        &mut model,
        FolderRowModel::new("Root", String::new(), 0, false, false, true, true, true),
    );
    push_active_folder_row(
        &mut model,
        FolderRowModel::rename_draft(
            1,
            String::from("Drums"),
            String::from("Folder name"),
            None,
            true,
        ),
    );
    let mut state = NativeShellState::new();
    let disclosure = state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("rename draft row should still have layout geometry");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, point),
        None
    );
}
