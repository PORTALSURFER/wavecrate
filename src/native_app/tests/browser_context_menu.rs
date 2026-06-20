use super::*;

#[test]
fn folder_context_menu_paints_as_full_width_overlay_panel() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        source_removable: false,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

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
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        source_removable: false,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let mut runtime = ui::DeclarativeOwnedSurfaceRuntime::new_declarative_owned(
        true,
        Vector2::new(960.0, 540.0),
        move |open| {
            if *open {
                ui::scene(ui::empty())
                    .layer(
                        radiant::Layer::context_menu(
                            crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu),
                        )
                        .dismiss_on_outside_click(
                            crate::native_app::test_support::state::GuiMessage::CloseContextMenu,
                        ),
                    )
                    .into_view()
                    .into_surface()
            } else {
                ui::empty().into_surface()
            }
        },
        |open, message| {
            if matches!(
                message,
                crate::native_app::test_support::state::GuiMessage::CloseContextMenu
            ) {
                *open = false;
            }
        },
    );
    apply_strict_update_diagnostics(&mut runtime);
    let outside_menu = Point::new(18.0, 18.0);

    runtime.dispatch_primary_click(outside_menu);

    assert!(
        !*runtime.bridge().state(),
        "clicking outside the context menu should route to the dismiss layer"
    );
}

#[test]
fn source_context_menu_paints_remove_source_action_for_user_sources() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Source,
        path: PathBuf::from("C:\\Samples"),
        source_id: Some(String::from("source_id::samples")),
        source_removable: true,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Samples"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Refresh Source"));
    assert!(frame.paint_plan.contains_text("Process Source"));
    assert!(frame.paint_plan.contains_text("New Folder"));
    assert!(!frame.paint_plan.contains_text("Delete Folder"));
    assert!(frame.paint_plan.contains_text("Remove Source"));
}

#[test]
fn source_context_menu_paints_refresh_for_default_sources_without_remove() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Source,
        path: PathBuf::from("C:\\Wavecrate\\assets"),
        source_id: Some(String::from("assets")),
        source_removable: false,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Assets"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Refresh Source"));
    assert!(frame.paint_plan.contains_text("Process Source"));
    assert!(frame.paint_plan.contains_text("New Folder"));
    assert!(!frame.paint_plan.contains_text("Delete Folder"));
    assert!(!frame.paint_plan.contains_text("Remove Source"));
}

#[test]
fn source_context_menu_processes_context_source_without_selecting_it() {
    let first_root = tempfile::tempdir().expect("first source root");
    let second_root = tempfile::tempdir().expect("second source root");
    fs::write(first_root.path().join("first.wav"), []).expect("write first sample");
    fs::write(second_root.path().join("second.wav"), []).expect("write second sample");
    let first_source_id = String::from("first-source");
    let second_source_id = String::from("second-source");
    let first_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(first_source_id.clone()),
        first_root.path().to_path_buf(),
    );
    let second_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(second_source_id.clone()),
        second_root.path().to_path_buf(),
    );
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            first_source,
            second_source,
        ]);
    let second_scan = state
        .library
        .folder_browser
        .begin_source_scan(second_source_id.clone(), 42)
        .expect("second source scan should queue");
    let second_scan_result =
        crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
            second_scan,
            |_| {},
            |_| {},
        );
    assert!(
        state
            .library
            .folder_browser
            .apply_scan_finished(second_scan_result)
    );
    assert_eq!(
        state.library.folder_browser.selected_source_id(),
        first_source_id
    );
    state.open_source_context_menu(second_source_id.clone(), Point::new(40.0, 120.0));
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ProcessContextSource,
        &mut context,
    );

    assert!(state.ui.browser_interaction.context_menu.is_none());
    assert_eq!(
        state.library.folder_browser.selected_source_id(),
        first_source_id
    );
    assert_eq!(
        state.waveform.cache.active_folder_warm_folder_id.as_deref(),
        Some(second_root.path().to_string_lossy().as_ref())
    );
    assert_eq!(state.waveform.cache.active_folder_warm_total, 1);
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_plan_task
            .active()
            .is_some(),
        "processing a context source should queue cache warm planning for that source"
    );
}

#[test]
fn folder_context_menu_paints_new_folder_action() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
        path: PathBuf::from("C:\\Samples\\Drums"),
        source_id: None,
        source_removable: false,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Drums"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("New Folder"));
    assert!(frame.paint_plan.contains_text("Rename Folder"));
    assert!(frame.paint_plan.contains_text("Delete Folder"));
}

#[test]
fn folder_context_menu_rename_starts_inline_rename_for_context_folder() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-context-menu-rename-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested folder");

    let mut state = gui_state_for_span_tests();
    let request = state
        .library
        .folder_browser
        .begin_add_source_path(root.clone(), 100)
        .expect("new source should request scan");
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());
    state.open_folder_context_menu(drums.to_string_lossy().to_string(), Point::new(40.0, 120.0));
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::RenameContextFolder,
        &mut context,
    );

    let target = state.library.folder_browser.selected_rename_target();
    assert_eq!(target.kind, "folder");
    assert_eq!(target.label, "drums");
    assert!(state.library.folder_browser.rename_active());
    assert_eq!(state.ui.status.sample, "Renaming selected folder");
    assert!(state.ui.browser_interaction.context_menu.is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sample_context_menu_paints_remove_from_collection_action_in_collection_view() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Sample,
        path: PathBuf::from("C:\\Samples\\kick.wav"),
        source_id: None,
        source_removable: false,
        metadata_tag: None,
        collection: wavecrate::sample_sources::SampleCollection::new(0),
        anchor: Point::new(72.0, 142.0),
        title: String::from("kick.wav"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Remove from collection"));
    assert!(!frame.paint_plan.contains_text("New Folder"));
    assert!(!frame.paint_plan.contains_text("Delete Folder"));
    assert!(frame.paint_plan.contains_text("Move to Trash"));
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
        .library
        .folder_browser
        .begin_add_source_path(root.clone(), 100)
        .expect("new source should request scan");
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());
    let (folder_id, expanded_before) = state
        .library
        .folder_browser
        .first_visible_child_folder_expansion_for_tests()
        .expect("test source should contain a child folder");

    state.open_folder_context_menu(folder_id.clone(), Point::new(40.0, 120.0));

    let expanded_after = state
        .library
        .folder_browser
        .folder_expansion_for_tests(&folder_id)
        .expect("context-menu target should remain visible");
    assert_eq!(
        expanded_after, expanded_before,
        "right-click context menu should not expand or collapse folders"
    );
    let _ = fs::remove_dir_all(root);
}
