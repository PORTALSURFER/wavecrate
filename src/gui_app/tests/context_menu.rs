use super::*;

#[test]
fn folder_context_menu_paints_as_full_width_overlay_panel() {
    let menu = super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        metadata_tag: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let frame =
        radiant::runtime::UiSurface::new(super::super::context_menu::overlay(&menu).into_node())
            .frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    let action_text_rect = frame
        .paint_plan
        .first_text_run("Open in Explorer")
        .map(|text| text.rect)
        .expect("folder context menu action text should render");

    assert!(action_text_rect.width() > 150.0, "{action_text_rect:?}");
    assert!(
        action_text_rect.min.x >= 80.0 && action_text_rect.min.x < 100.0,
        "{action_text_rect:?}"
    );
}

#[test]
fn folder_context_menu_outside_click_closes_menu() {
    let menu = super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        metadata_tag: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        true,
        move |open| {
            if *open {
                radiant::runtime::UiSurface::new(
                    super::super::context_menu::overlay(&menu).into_node(),
                )
            } else {
                radiant::runtime::UiSurface::new(ui::text("").into_node())
            }
        },
        |open, message| {
            if matches!(message, super::super::GuiMessage::CloseContextMenu) {
                *open = false;
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(960.0, 540.0));
    let outside_menu = Point::new(18.0, 18.0);

    runtime.dispatch_primary_click(outside_menu);

    assert!(
        !*runtime.bridge().state(),
        "clicking outside the context menu should route to the dismiss layer"
    );
}

#[test]
fn source_context_menu_paints_remove_source_action_for_user_sources() {
    let menu = super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Source,
        path: PathBuf::from("C:\\Samples"),
        source_id: Some(String::from("source_id::samples")),
        metadata_tag: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Samples"),
    };
    let frame =
        radiant::runtime::UiSurface::new(super::super::context_menu::overlay(&menu).into_node())
            .frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Remove Source"));
}

#[test]
fn folder_context_menu_open_does_not_toggle_folder_expansion() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-context-menu-right-click-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let parent = root.join("drums");
    fs::create_dir_all(parent.join("kicks")).expect("create nested folder");

    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(root.clone(), 100)
        .expect("new source should request scan");
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result);
    let (folder_id, expanded_before) = state
        .folder_browser
        .first_visible_child_folder_expansion_for_tests()
        .expect("test source should contain a child folder");

    state.open_folder_context_menu(folder_id.clone(), Point::new(40.0, 120.0));

    let expanded_after = state
        .folder_browser
        .folder_expansion_for_tests(&folder_id)
        .expect("context-menu target should remain visible");
    assert_eq!(
        expanded_after, expanded_before,
        "right-click context menu should not expand or collapse folders"
    );
    let _ = fs::remove_dir_all(root);
}
