use super::*;

#[test]
fn waveform_toolbar_renders_svg_backed_icon_images() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.transport_running = false;
    let mut state = NativeShellState::new();
    let play_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Play")
        .expect("play waveform toolbar button should be present");
    let channel_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Channel")
        .expect("channel waveform toolbar button should be present");

    let frame = state.build_frame(&layout, &model);

    for button_rect in [play_rect, channel_rect] {
        let image = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Image(image)
                    if image.rect.min.x >= button_rect.min.x
                        && image.rect.min.y >= button_rect.min.y
                        && image.rect.max.x <= button_rect.max.x
                        && image.rect.max.y <= button_rect.max.y =>
                {
                    Some(image)
                }
                _ => None,
            })
            .expect("toolbar button should render an SVG-backed image primitive");
        assert!(
            image.image.pixels.chunks_exact(4).any(|rgba| rgba[3] > 0),
            "toolbar SVG image should contain visible pixels"
        );
    }
}

#[test]
fn options_panel_omits_inner_title_drag_surface() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
            visible: true,
            ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };
    let initial = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let grab = initial.title_rect.center();

    assert!(initial.title.is_empty());
    assert_eq!(initial.title_rect.height(), 0.0);
    assert!(!state.begin_options_panel_drag(&layout, &model, grab));
}

#[test]
fn options_panel_drag_clamps_window_inside_shell_bounds() {
    let layout = ShellLayout::build(Vector2::new(900.0, 520.0));
    let style = style_for_layout(&layout);
    let model = AppModel {
        options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
            visible: true,
            ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };

    let panel = options_panel_layout_for_origin(
        &layout,
        &style,
        &model,
        Some(Point::new(-10_000.0, 10_000.0)),
    )
    .expect("visible options panel should resolve layout");

    assert!(panel.panel_rect.min.x >= layout.root.rect.min.x);
    assert!(panel.panel_rect.min.y >= layout.top_bar.max.y);
    assert!(panel.panel_rect.max.x <= layout.root.rect.max.x);
    assert!(panel.panel_rect.max.y <= layout.status_bar.min.y);
}

#[test]
fn options_panel_picker_mode_expands_inline_dropdown_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
            visible: true,
            ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
        },
        paired_device: crate::app_core::native_shell::runtime_contract::PairedDevicePanelModel {
            primary_number: crate::app_core::native_shell::runtime_contract::SummaryFieldModel {
                label: String::from("Sample Rate"),
                value_label: String::from("48 kHz"),
            },
            active_picker: Some(
                crate::app_core::native_shell::runtime_contract::PairedPickerTargetModel::PrimaryNumber,
            ),
            primary_number_options: vec![
                crate::app_core::native_shell::runtime_contract::PairedPickerOptionModel {
                    label: String::from("Device default"),
                    selected: false,
                    value: crate::app_core::native_shell::runtime_contract::PairedPickerValueModel::PrimaryNumber(None),
                },
                crate::app_core::native_shell::runtime_contract::PairedPickerOptionModel {
                    label: String::from("48 kHz"),
                    selected: true,
                    value: crate::app_core::native_shell::runtime_contract::PairedPickerValueModel::PrimaryNumber(Some(
                        48_000,
                    )),
                },
            ],
            ..crate::app_core::native_shell::runtime_contract::PairedDevicePanelModel::default()
        },
        ..AppModel::default()
    };

    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible picker panel should resolve layout");
    assert!(panel.title.is_empty());
    let dropdown_row = panel
        .buttons
        .iter()
        .position(|button| button.action == UiAction::OpenPrimaryNumberPicker)
        .expect("active picker row should remain in the overview");
    assert!(panel.buttons[dropdown_row].active);
    assert!(panel.buttons[dropdown_row].text.starts_with("Sample Rate"));
    assert_eq!(
        panel.buttons[dropdown_row + 1].action,
        UiAction::SetPrimaryNumber { value: None }
    );
    assert_eq!(
        panel.buttons[dropdown_row + 2].action,
        UiAction::SetPrimaryNumber {
            value: Some(48_000),
        }
    );
    assert!(panel.buttons[dropdown_row + 2].active);

    let dropdown_point = panel.buttons[dropdown_row].rect.center();
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, dropdown_point),
        Some(UiAction::OpenPrimaryNumberPicker)
    );

    let sample_rate_point = panel.buttons[dropdown_row + 2].rect.center();
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, sample_rate_point),
        Some(UiAction::SetPrimaryNumber {
            value: Some(48_000),
        })
    );
}
