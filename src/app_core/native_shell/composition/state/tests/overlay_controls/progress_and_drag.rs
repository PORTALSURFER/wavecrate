use super::*;
#[test]
fn non_modal_progress_renders_status_bar_indicator_without_overlay_dialog() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Normalizing sample"),
            detail: Some(String::from("kick.wav")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);
    let overlay_rect = compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        false,
        0.4,
    )
    .sections
    .dialog;

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Normalizing sample")),
        "status bar should announce the active job title"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "2/5"),
        "status bar should show progress counts"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "col: 2/3"),
        "status bar should keep the right-side status text visible"
    );
    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x >= layout.status_progress_segment.min.x
                    && rect.rect.max.x <= layout.status_progress_segment.max.x
                    && rect.color == style.accent_mint
        )),
        "status bar should render an inline progress fill"
    );
    assert!(
        !frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect == overlay_rect && rect.color == style.surface_overlay
        )),
        "non-modal jobs should not render the floating overlay dialog"
    );
}

#[test]
fn non_modal_progress_does_not_expose_cancel_hit_target() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Normalizing"),
            completed: 0,
            total: 1,
            cancelable: true,
            cancel_requested: false,
            ..crate::compat_app_contract::ProgressOverlayModel::default()
        },
        ..AppModel::default()
    };
    let cancel_button = progress_cancel_button(&layout, &style, false);
    let point = Point::new(
        (cancel_button.min.x + cancel_button.max.x) * 0.5,
        (cancel_button.min.y + cancel_button.max.y) * 0.5,
    );

    assert_eq!(state.progress_action_at_point(&layout, &model, point), None);
}

#[test]
fn modal_progress_overlay_renders_cancelling_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: true,
            title: String::from("Normalizing"),
            detail: Some(String::from("kick.wav")),
            completed: 1,
            total: 4,
            cancelable: true,
            cancel_requested: true,
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.text_runs.iter().any(|run| run.text == "Cancelling"));
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == progress_cancel_button(&layout, &style, true)
                && rect.color == style.grid_soft
    )));
}

#[test]
fn confirm_prompt_validation_error_renders_disabled_confirm_button_and_error_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        confirm_prompt: crate::compat_app_contract::ConfirmPromptModel {
            visible: true,
            title: String::from("Rename"),
            message: String::from("Choose a new name"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Dismiss"),
            input_value: Some(String::from("bad/name")),
            input_error: Some(String::from("Slash not allowed")),
            ..crate::compat_app_contract::ConfirmPromptModel::default()
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Slash not allowed" && run.color == style.accent_warning)
    );
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if rect.color == style.control_disabled_fill
    )));
}

#[test]
fn drag_overlay_renders_target_arrow_and_warning_text_for_invalid_drop() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        drag_overlay: crate::compat_app_contract::DragOverlayModel {
            active: true,
            label: String::from("kick.wav"),
            target_label: String::from("Trash"),
            valid_target: false,
            pointer_x: Some(420),
            pointer_y: Some(240),
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| { run.text == "kick.wav -> Trash" && run.color == style.accent_warning })
    );
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == drag_overlay_rect(&layout, &style)
                && rect.color == style.surface_overlay
    )));
    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "kick.wav" && run.color == style.accent_warning)
    );
}

#[test]
fn drag_overlay_renders_cursor_chip_near_pointer() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        drag_overlay: crate::compat_app_contract::DragOverlayModel {
            active: true,
            label: String::from("2 samples"),
            target_label: String::from("Folder: drums"),
            valid_target: true,
            pointer_x: Some(300),
            pointer_y: Some(210),
        },
        ..AppModel::default()
    };

    let chip = drag_chip_rect(&layout, &style, &model).expect("drag chip should resolve");
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert_rect_inside(
        Rect::from_min_max(
            Point::new(layout.root.rect.min.x, layout.top_bar.max.y),
            Point::new(layout.root.rect.max.x, layout.status_bar.min.y),
        ),
        chip,
    );
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if rect.rect == chip
    )));
    assert!(overlay.text_runs.iter().any(|run| run.text == "2 samples"));
}

#[test]
fn drag_overlay_chip_flips_and_clamps_near_body_edges() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = AppModel {
        drag_overlay: crate::compat_app_contract::DragOverlayModel {
            active: true,
            label: String::from("kick.wav"),
            target_label: String::from("Folder: drums"),
            valid_target: true,
            pointer_x: Some(1272),
            pointer_y: Some(652),
        },
        ..AppModel::default()
    };

    let chip = drag_chip_rect(&layout, &style, &model).expect("drag chip should resolve");
    let body_bounds = Rect::from_min_max(
        Point::new(layout.root.rect.min.x, layout.top_bar.max.y),
        Point::new(layout.root.rect.max.x, layout.status_bar.min.y),
    );

    assert_rect_inside(body_bounds, chip);
    assert!(chip.min.x < f32::from(model.drag_overlay.pointer_x.expect("pointer x")));
    assert!(chip.max.y <= body_bounds.max.y);
}

#[test]
fn state_overlay_renders_options_panel_when_visible() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Audio Engine")
    );
}
