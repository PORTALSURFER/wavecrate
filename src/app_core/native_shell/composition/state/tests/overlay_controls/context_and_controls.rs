use super::*;

#[test]
/// Source context menu hit testing should emit reload for the targeted row.
fn source_context_menu_hit_test_emits_reload_action_for_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    model
        .sources
        .rows
        .get_mut(0)
        .expect("source row should exist")
        .assigned_to_upper_pane = true;
    let row_rect = *state
        .rendered_source_row_rects(&layout, &model)
        .first()
        .expect("source row should be rendered");
    let anchor = Point::new(
        (row_rect.min.x + row_rect.max.x) * 0.5,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    );
    state.open_source_context_menu_for_row(
        crate::compat_app_contract::FolderPaneIdModel::Upper,
        0,
        anchor,
    );

    let reload_rect = state
        .source_context_menu_button_rect(
            &layout,
            &model,
            UiAction::ReloadSourceRow {
                pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                index: 0,
            },
        )
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::ReloadSourceRow {
            pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
            index: 0,
        })
    );
}

#[test]
/// Source context menu geometry should disappear after explicit close.
fn source_context_menu_contains_point_tracks_open_close_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    model
        .sources
        .rows
        .get_mut(0)
        .expect("source row should exist")
        .assigned_to_upper_pane = true;
    state.open_source_context_menu_for_row(
        crate::compat_app_contract::FolderPaneIdModel::Upper,
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );
    let reload_rect = state
        .source_context_menu_button_rect(
            &layout,
            &model,
            UiAction::ReloadSourceRow {
                pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                index: 0,
            },
        )
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert!(state.source_context_menu_contains_point(&layout, &model, point));
    assert!(state.close_source_context_menu());
    assert!(!state.source_context_menu_contains_point(&layout, &model, point));
}

#[test]
/// Source context menu should expose source removal and render in the overlay pass.
fn source_context_menu_exposes_remove_action_in_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    model
        .sources
        .rows
        .get_mut(0)
        .expect("source row should exist")
        .assigned_to_upper_pane = true;
    state.open_source_context_menu_for_row(
        crate::compat_app_contract::FolderPaneIdModel::Upper,
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );

    let remove_rect = state
        .source_context_menu_button_rect(
            &layout,
            &model,
            UiAction::RemoveSourceRow {
                pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                index: 0,
            },
        )
        .expect("remove source action button should be present");
    let point = Point::new(
        (remove_rect.min.x + remove_rect.max.x) * 0.5,
        (remove_rect.min.y + remove_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::RemoveSourceRow {
            pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
            index: 0,
        })
    );

    let frame = state.build_frame(&layout, &model);
    assert!(
        !frame
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );
}

#[test]
fn browser_context_menu_exposes_auto_rename_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "kick.wav", 1, true, true));
    let row_rect = state
        .cached_browser_rows(&layout, &style_for_layout(&layout), &model)
        .first()
        .expect("browser row should be rendered")
        .rect;
    let anchor = Point::new(
        (row_rect.min.x + row_rect.max.x) * 0.5,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    );
    state.open_browser_context_menu_for_row(0, anchor);

    let button_rect = state
        .browser_context_menu_button_rect(
            &layout,
            &model,
            UiAction::AutoRenameBrowserSelection {
                visible_row: Some(0),
            },
        )
        .expect("auto rename action button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert!(state.browser_context_menu_contains_point(&layout, &model, point));
    assert_eq!(
        state.browser_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::AutoRenameBrowserSelection {
            visible_row: Some(0),
        })
    );
}

#[test]
fn tick_with_style_uses_tier_motion_speed_tokens() {
    let mut model = AppModel::default();
    model.transport_running = true;
    let compact_style = StyleTokens::for_viewport_width(820.0);
    let wide_style = StyleTokens::for_viewport_width(2300.0);

    let mut compact_state = NativeShellState::new();
    compact_state.sync_from_model(&model);
    compact_state.tick_with_style(1.0, &compact_style);

    let mut wide_state = NativeShellState::new();
    wide_state.sync_from_model(&model);
    wide_state.tick_with_style(1.0, &wide_style);

    assert!(compact_state.pulse_phase > 0.0);
    assert!(wide_state.pulse_phase > compact_state.pulse_phase);
}

#[test]
fn top_bar_volume_click_maps_to_set_volume_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let model = AppModel::default();
    let controls = resolve_top_bar_surface_layout(
        layout.top_bar,
        style_for_layout(&layout).sizing,
        &top_bar_surface_content(&model),
    );
    assert!(controls.volume_meter_rect.width() > 0.0);
    let point = Point::new(
        controls.volume_meter_rect.min.x + (controls.volume_meter_rect.width() * 0.75),
        controls.volume_meter_rect.min.y + (controls.volume_meter_rect.height() * 0.5),
    );
    let action = state
        .top_bar_volume_action_at_point(&layout, &model, point)
        .expect("volume click should produce action");
    assert_eq!(action, UiAction::SetVolume { value_milli: 750 });
}

#[test]
fn status_options_click_maps_to_open_options_menu_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let button = state
        .status_options_button_rect(&layout, &model)
        .expect("status options button should render");
    let point = Point::new(
        button.min.x + (button.width() * 0.5),
        button.min.y + (button.height() * 0.5),
    );
    let action = state
        .status_options_action_at_point(&layout, &model, point)
        .expect("options click should produce action");
    assert_eq!(action, UiAction::OpenOptionsMenu);
}

#[test]
fn options_panel_contains_points_inside_panel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };
    let point = Point::new(layout.top_bar.max.x - 40.0, layout.top_bar.max.y + 40.0);
    assert!(state.options_panel_contains_point(&layout, &model, point));
}

#[test]
fn options_panel_trash_folder_buttons_emit_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            trash_folder_label: Some(String::from("trash_bin")),
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };
    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let set_button = panel
        .buttons
        .iter()
        .find(|button| button.action == UiAction::PickTrashFolder)
        .expect("set trash folder button should be present");
    let set_point = Point::new(
        (set_button.rect.min.x + set_button.rect.max.x) * 0.5,
        (set_button.rect.min.y + set_button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, set_point),
        Some(UiAction::PickTrashFolder)
    );

    let open_button = panel
        .buttons
        .iter()
        .find(|button| button.action == UiAction::OpenTrashFolder)
        .expect("open trash folder button should be present");
    let open_point = Point::new(
        (open_button.rect.min.x + open_button.rect.max.x) * 0.5,
        (open_button.rect.min.y + open_button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, open_point),
        Some(UiAction::OpenTrashFolder)
    );
}

#[test]
fn options_panel_default_identifier_button_emits_edit_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            default_identifier: String::from("portal"),
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };

    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let button = panel
        .buttons
        .iter()
        .find(|button| button.action == UiAction::EditDefaultIdentifier)
        .expect("default identifier button should be present");
    let point = Point::new(
        (button.rect.min.x + button.rect.max.x) * 0.5,
        (button.rect.min.y + button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, point),
        Some(UiAction::EditDefaultIdentifier)
    );
}

#[test]
fn status_options_chip_renders_audio_label_and_error_tint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        paired_device: crate::compat_app_contract::PairedDevicePanelModel {
            status_state: crate::compat_app_contract::StatusChipStateModel::Error,
            status_label: String::from("Audio Err"),
            ..crate::compat_app_contract::PairedDevicePanelModel::default()
        },
        ..AppModel::default()
    };
    state.sync_from_model(&model);

    let button = state
        .status_options_button_rect(&layout, &model)
        .expect("status options chip should render");
    let frame = state.build_frame(&layout, &model);

    assert!(button.width() > button.height());
    assert!(frame.text_runs.iter().any(|run| run.text == "Audio Err"));
    let fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == button => Some(*color),
            _ => None,
        })
        .expect("chip fill should be rendered");
    assert_ne!(fill, style.surface_overlay);
}

#[test]
fn top_bar_update_buttons_emit_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let model = AppModel {
        update: crate::compat_app_contract::UpdatePanelModel {
            status: crate::compat_app_contract::UpdateStatusModel::Available,
            status_label: String::from("Update available: v20.1.0"),
            action_hint_label: String::from("Actions: open | install(manual) | dismiss"),
            release_notes_label: String::from("Release: v20.1.0"),
            available_version_label: Some(String::from("v20.1.0")),
            available_url: Some(String::from("https://example.invalid/release")),
            last_error: None,
        },
        ..AppModel::default()
    };

    for action in [
        UiAction::OpenUpdateLink,
        UiAction::InstallUpdate,
        UiAction::DismissUpdate,
    ] {
        let button = state
            .top_bar_update_button_rect(&layout, &model, action.clone())
            .expect("update action should resolve button rect");
        let point = Point::new(
            (button.min.x + button.max.x) * 0.5,
            (button.min.y + button.max.y) * 0.5,
        );
        assert_eq!(
            state.top_bar_update_action_at_point(&layout, &model, point),
            Some(action)
        );
    }
}

#[test]
fn options_panel_overview_lists_audio_rows_before_legacy_toggles() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        paired_device: crate::compat_app_contract::PairedDevicePanelModel {
            primary_group: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Output Host"),
                value_label: String::from("ASIO"),
            },
            primary_item: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Output Device"),
                value_label: String::from("USB"),
            },
            primary_number: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Output Sample Rate"),
                value_label: String::from("48 kHz"),
            },
            secondary_group: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Input Host"),
                value_label: String::from("WASAPI"),
            },
            secondary_item: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Input Device"),
                value_label: String::from("Mic"),
            },
            secondary_number: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Input Sample Rate"),
                value_label: String::from("44.1 kHz"),
            },
            ..crate::compat_app_contract::PairedDevicePanelModel::default()
        },
        ..AppModel::default()
    };

    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let actions = panel
        .buttons
        .iter()
        .map(|button| button.action.clone())
        .collect::<Vec<_>>();

    assert_eq!(panel.title, "Audio Engine");
    assert_eq!(
        actions[..6],
        [
            UiAction::OpenPrimaryGroupPicker,
            UiAction::OpenPrimaryItemPicker,
            UiAction::OpenPrimaryNumberPicker,
            UiAction::OpenSecondaryGroupPicker,
            UiAction::OpenSecondaryItemPicker,
            UiAction::OpenSecondaryNumberPicker,
        ]
    );
    assert_eq!(actions.last(), Some(&UiAction::CloseOptionsPanel));
}

#[test]
fn options_panel_picker_mode_uses_back_row_and_picker_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        paired_device: crate::compat_app_contract::PairedDevicePanelModel {
            active_picker: Some(crate::compat_app_contract::PairedPickerTargetModel::PrimaryNumber),
            primary_number_options: vec![
                crate::compat_app_contract::PairedPickerOptionModel {
                    label: String::from("Device default"),
                    selected: false,
                    value: crate::compat_app_contract::PairedPickerValueModel::PrimaryNumber(None),
                },
                crate::compat_app_contract::PairedPickerOptionModel {
                    label: String::from("48 kHz"),
                    selected: true,
                    value: crate::compat_app_contract::PairedPickerValueModel::PrimaryNumber(Some(
                        48_000,
                    )),
                },
            ],
            ..crate::compat_app_contract::PairedDevicePanelModel::default()
        },
        ..AppModel::default()
    };

    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible picker panel should resolve layout");
    assert_eq!(panel.title, "Output Sample Rate");
    assert_eq!(panel.buttons[0].action, UiAction::ShowOptionsOverview);
    assert!(panel.buttons[2].active);

    let back_point = Point::new(
        (panel.buttons[0].rect.min.x + panel.buttons[0].rect.max.x) * 0.5,
        (panel.buttons[0].rect.min.y + panel.buttons[0].rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, back_point),
        Some(UiAction::ShowOptionsOverview)
    );

    let sample_rate_button = &panel.buttons[2];
    let sample_rate_point = Point::new(
        (sample_rate_button.rect.min.x + sample_rate_button.rect.max.x) * 0.5,
        (sample_rate_button.rect.min.y + sample_rate_button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, sample_rate_point),
        Some(UiAction::SetPrimaryNumber {
            value: Some(48_000),
        })
    );
}

#[test]
fn options_panel_renders_after_other_modal_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::compat_app_contract::OptionsPanelModel {
            visible: true,
            ..crate::compat_app_contract::OptionsPanelModel::default()
        },
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: true,
            title: String::from("Background task"),
            detail: Some(String::from("Copying files")),
            completed: 1,
            total: 3,
            cancelable: true,
            cancel_requested: false,
        },
        paired_device: crate::compat_app_contract::PairedDevicePanelModel {
            status_label: String::from("48 kHz"),
            primary_group: crate::compat_app_contract::SummaryFieldModel {
                label: String::from("Output Host"),
                value_label: String::from("ASIO"),
            },
            ..crate::compat_app_contract::PairedDevicePanelModel::default()
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    let progress_title_index = overlay
        .text_runs
        .iter()
        .position(|run| run.text == "Background task")
        .expect("progress overlay title should render");
    let panel_title_index = overlay
        .text_runs
        .iter()
        .position(|run| run.text == "Audio Engine")
        .expect("options panel title should render");
    assert!(panel_title_index > progress_title_index);

    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let progress_rect_index = overlay
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, .. }) if *rect != panel.panel_rect
                    && rect.width() > panel.panel_rect.width() * 0.5
                    && rect.height() > panel.panel_rect.height() * 0.5
            )
        })
        .expect("progress overlay backdrop should render");
    let panel_rect_index = overlay
        .primitives
        .iter()
        .rposition(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, .. }) if *rect == panel.panel_rect
            )
        })
        .expect("options panel surface should render");
    assert!(panel_rect_index > progress_rect_index);
}
