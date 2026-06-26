use super::*;
use crate::native_app::test_support::state::GuiMessage;

#[test]
fn folder_context_menu_paints_as_full_width_overlay_panel() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
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
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
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
fn playmark_context_menu_paints_selection_actions() {
    let menu = crate::native_app::test_support::context_menu::WaveformContextMenu {
        anchor: Point::new(240.0, 180.0),
        title: String::from("Playmark Selection"),
        extract_to_harvest_destination: false,
    };
    let frame = crate::native_app::test_support::context_menu::waveform_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    for label in [
        "Play Selection",
        "Extract Selection",
        "Extract and Trim",
        "Crop to Selection",
        "Trim Selection",
        "Reverse Selection",
        "Zoom to Selection",
        "Find Similar Sections",
    ] {
        assert!(
            frame.paint_plan.contains_text(label),
            "{label} should render in the playmark context menu"
        );
    }
}

#[test]
fn playmark_context_menu_labels_harvest_destination_extraction() {
    let menu = crate::native_app::test_support::context_menu::WaveformContextMenu {
        anchor: Point::new(240.0, 180.0),
        title: String::from("Playmark Selection"),
        extract_to_harvest_destination: true,
    };
    let frame = crate::native_app::test_support::context_menu::waveform_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(
        frame
            .paint_plan
            .contains_text("Extract to Harvest Destination")
    );
    assert!(
        frame.paint_plan.contains_text("Extract Selection"),
        "normal extraction should remain available beside the harvest route"
    );
}

#[test]
fn w_command_opens_playmark_context_menu_and_clears_browser_menu() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.2, 0.6);
    state.ui.browser_interaction.context_menu = Some(
        crate::native_app::test_support::context_menu::BrowserContextMenu {
            kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
            path: PathBuf::from("Documents"),
            source_id: None,
            source_role: wavecrate::sample_sources::SourceRole::Normal,
            source_removable: false,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection: None,
            sample_missing: false,
            anchor: Point::new(72.0, 142.0),
            title: String::from("Documents"),
        },
    );

    state.apply_message(
        GuiMessage::OpenPlaySelectionContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.ui.browser_interaction.context_menu, None);
    assert_eq!(
        state.ui.browser_interaction.waveform_context_menu,
        Some(
            crate::native_app::test_support::context_menu::WaveformContextMenu {
                anchor: Point::new(240.0, 160.0),
                title: String::from("Playmark Selection"),
                extract_to_harvest_destination: false,
            }
        )
    );
}

#[test]
fn w_command_opens_playmark_context_menu_from_current_selection() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.2, 0.6);

    state.apply_message(
        GuiMessage::OpenPlaySelectionContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    let menu = state
        .ui
        .browser_interaction
        .waveform_context_menu
        .expect("playmark context menu opens");
    assert_eq!(menu.title, "Playmark Selection");
    assert!((menu.anchor.x - 240.0).abs() < 0.001);
    assert!((menu.anchor.y - 160.0).abs() < 0.001);
}

#[test]
fn w_command_opens_playmark_context_menu_at_remembered_mouse_location() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.2, 0.6);
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::RememberPointerLocation {
            position: Point::new(333.0, 222.0),
        });

    state.apply_message(
        GuiMessage::OpenPlaySelectionContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    let menu = state
        .ui
        .browser_interaction
        .waveform_context_menu
        .expect("playmark context menu opens");
    assert_eq!(menu.anchor, Point::new(333.0, 222.0));
}

#[test]
fn waveform_interaction_marks_context_menu_harvest_destination_route() {
    let protected_root = tempfile::tempdir().expect("protected source root");
    let primary_root = tempfile::tempdir().expect("primary source root");
    let sample = protected_root.path().join("harvest-playmark.wav");
    write_test_wav_i16(&sample, &[0, 1024, -1024, 512]);
    let protected_source =
        wavecrate::sample_sources::SampleSource::new(protected_root.path().to_path_buf())
            .protected();
    let primary_source =
        wavecrate::sample_sources::SampleSource::new(primary_root.path().to_path_buf()).primary();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            protected_source,
            primary_source,
        ]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(sample)
            .expect("sample loads");
    state.waveform.current.set_play_selection_range(0.2, 0.6);

    state.apply_message(
        GuiMessage::OpenPlaySelectionContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .ui
            .browser_interaction
            .waveform_context_menu
            .as_ref()
            .map(|menu| menu.extract_to_harvest_destination),
        Some(true)
    );
}

#[test]
fn waveform_interaction_marks_harvest_destination_route_for_normal_harvest_mode() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("harvest-playmark-normal.wav");
    write_test_wav_i16(&sample, &[0, 1024, -1024, 512]);
    let source = wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf());
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::NeedsReview,
        true,
    );
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(sample)
            .expect("sample loads");
    state.waveform.current.set_play_selection_range(0.2, 0.6);

    state.apply_message(
        GuiMessage::OpenPlaySelectionContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .ui
            .browser_interaction
            .waveform_context_menu
            .as_ref()
            .map(|menu| menu.extract_to_harvest_destination),
        Some(true)
    );
}

#[test]
fn source_context_menu_paints_remove_source_action_for_user_sources() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Source,
        path: PathBuf::from("C:\\Samples"),
        source_id: Some(String::from("source_id::samples")),
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: true,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
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
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
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
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Drums"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("New Folder"));
    assert!(frame.paint_plan.contains_text("Rename Folder"));
    assert!(frame.paint_plan.contains_text("Lock Folder"));
    assert!(frame.paint_plan.contains_text("Delete Folder"));
}

#[test]
fn folder_context_menu_commands_share_neutral_style() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Folder,
        path: PathBuf::from("C:\\Samples\\Drums"),
        source_id: None,
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Drums"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    let command_labels = [
        "Open in Explorer",
        "Copy Path",
        "New Folder",
        "Rename Folder",
        "Lock Folder",
        "Delete Folder",
    ];
    let expected_color = frame
        .paint_plan
        .first_text_run("New Folder")
        .expect("new-folder command text paints")
        .color;
    for label in command_labels {
        let color = frame
            .paint_plan
            .first_text_run(label)
            .unwrap_or_else(|| panic!("{label} command text paints"))
            .color;
        assert_eq!(color, expected_color, "{label} should use neutral text");
    }

    let danger = radiant::theme::ThemeTokens::default().accent_danger;
    assert!(
        !frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == danger),
        "folder menu should not paint a separate danger-colored command row"
    );
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
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: wavecrate::sample_sources::SampleCollection::new(0),
        sample_missing: false,
        anchor: Point::new(72.0, 142.0),
        title: String::from("kick.wav"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Remove from collection"));
    assert!(!frame.paint_plan.contains_text("New Folder"));
    assert!(!frame.paint_plan.contains_text("Delete Folder"));
    assert!(frame.paint_plan.contains_text("Mark Harvest Done"));
    assert!(frame.paint_plan.contains_text("Ignore in Harvest"));
    assert!(frame.paint_plan.contains_text("Reset Harvest"));
    assert!(frame.paint_plan.contains_text("Show Harvest Origin"));
    assert!(frame.paint_plan.contains_text("Show Harvest Derivatives"));
    assert!(frame.paint_plan.contains_text("Open Harvest Destination"));
    assert!(frame.paint_plan.contains_text("Move to Trash"));
}

#[test]
fn sample_context_menu_harvest_state_actions_update_durable_state() {
    let root = tempfile::tempdir().expect("source root");
    let sample = root.path().join("kick.wav");
    fs::write(&sample, [0_u8; 8]).expect("write sample");
    let source_id_text = format!(
        "harvest-context-menu-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    let source_id = wavecrate::sample_sources::SourceId::from_string(source_id_text);
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        source_id.clone(),
        root.path().to_path_buf(),
    );
    let harvest_key =
        wavecrate::sample_sources::HarvestFileKey::new(source_id, PathBuf::from("kick.wav"));
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    let mut context = ui::UiUpdateContext::default();

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::MarkContextSampleHarvestDone,
        &mut context,
    );
    assert_eq!(
        wavecrate::sample_sources::library::harvest_file(&harvest_key)
            .expect("load harvest row")
            .expect("harvest row")
            .state,
        wavecrate::sample_sources::HarvestState::Done
    );

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::MarkContextSampleHarvestIgnored,
        &mut context,
    );
    assert_eq!(
        wavecrate::sample_sources::library::harvest_file(&harvest_key)
            .expect("load harvest row")
            .expect("harvest row")
            .state,
        wavecrate::sample_sources::HarvestState::Ignored
    );

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ResetContextSampleHarvest,
        &mut context,
    );
    let record = wavecrate::sample_sources::library::harvest_file(&harvest_key)
        .expect("load harvest row")
        .expect("harvest row");
    assert_eq!(record.state, wavecrate::sample_sources::HarvestState::New);
    assert_eq!(record.file_size, Some(8));
}

#[test]
fn sample_context_menu_harvest_state_actions_refresh_active_queue_visibility() {
    let root = tempfile::tempdir().expect("source root");
    let sample = root.path().join("kick.wav");
    fs::write(&sample, [0_u8; 8]).expect("write sample");
    let source_id_text = format!(
        "harvest-context-menu-refresh-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(source_id_text),
        root.path().to_path_buf(),
    );
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    let mut context = ui::UiUpdateContext::default();

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::NeedsReview,
        true,
    );
    assert_eq!(visible_sample_names(&state), vec!["kick.wav"]);

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::MarkContextSampleHarvestDone,
        &mut context,
    );
    assert!(
        visible_sample_names(&state).is_empty(),
        "Done files should immediately leave the active harvest review queue"
    );

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::Done,
        true,
    );
    assert_eq!(visible_sample_names(&state), vec!["kick.wav"]);

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ResetContextSampleHarvest,
        &mut context,
    );
    assert!(
        visible_sample_names(&state).is_empty(),
        "Reset files should immediately leave the Done harvest queue"
    );

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::NeedsReview,
        true,
    );
    assert_eq!(visible_sample_names(&state), vec!["kick.wav"]);

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::MarkContextSampleHarvestIgnored,
        &mut context,
    );
    assert!(
        visible_sample_names(&state).is_empty(),
        "Ignored files should immediately leave the active harvest review queue"
    );

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::Ignored,
        true,
    );
    assert_eq!(visible_sample_names(&state), vec!["kick.wav"]);
    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::All,
        true,
    );
    assert_eq!(visible_sample_names(&state), vec!["kick.wav"]);
}

fn visible_sample_names(state: &NativeAppState) -> Vec<String> {
    state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.name.clone())
        .collect()
}

#[test]
fn sample_context_menu_open_harvest_destination_requires_primary_source() {
    let root = tempfile::tempdir().expect("source root");
    let sample = root.path().join("kick.wav");
    fs::write(&sample, [0_u8; 8]).expect("write sample");
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("harvest-destination-without-primary"),
        root.path().to_path_buf(),
    );
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    let mut context = ui::UiUpdateContext::default();

    state.open_sample_context_menu(sample.display().to_string(), Point::new(40.0, 120.0));
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::OpenContextSampleHarvestDestination,
        &mut context,
    );

    assert_eq!(
        state.ui.status.sample,
        "Set a Primary source before using a harvest destination"
    );
    assert!(state.ui.browser_interaction.context_menu.is_none());
}

#[test]
fn selected_harvest_family_summary_reports_state_origin_and_derivatives() {
    let origin_root = tempfile::tempdir().expect("origin root");
    let derived_root = tempfile::tempdir().expect("derived root");
    let origin_sample = origin_root.path().join("jam.wav");
    let child_sample = derived_root.path().join("jam chop 01.wav");
    fs::write(&origin_sample, [0_u8; 8]).expect("write origin");
    fs::write(&child_sample, [0_u8; 8]).expect("write child");
    let origin_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-family-origin-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let child_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-family-child-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let origin_source = wavecrate::sample_sources::SampleSource::new_with_id(
        origin_id.clone(),
        origin_root.path().to_path_buf(),
    );
    let child_source = wavecrate::sample_sources::SampleSource::new_with_id(
        child_id.clone(),
        derived_root.path().to_path_buf(),
    );
    let origin_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            origin_id.clone(),
            PathBuf::from("jam.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(1),
        content_hash: None,
    };
    let child_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            child_id.clone(),
            PathBuf::from("jam chop 01.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(2),
        content_hash: None,
    };
    wavecrate::sample_sources::library::record_harvest_derivation(
        &wavecrate::sample_sources::NewHarvestDerivation {
            parent: origin_identity.clone(),
            child: child_identity.clone(),
            operation: wavecrate::sample_sources::HarvestDerivationOperation::Extract,
            source_range: None,
            output_duration_seconds: None,
            destination_folder: Some(derived_root.path().to_path_buf()),
            inherited_metadata: wavecrate::sample_sources::HarvestMetadataSnapshot::default(),
            tool_version: String::from("wavecrate-test"),
        },
    )
    .expect("record edge");
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            origin_source,
            child_source,
        ]);

    state
        .library
        .folder_browser
        .select_file(origin_sample.display().to_string());
    let origin_summary = state
        .selected_harvest_family_summary()
        .expect("origin should have family summary");
    assert_eq!(origin_summary.state_label, "Touched");
    assert_eq!(origin_summary.origin_count, 0);
    assert_eq!(origin_summary.derivative_count, 1);
    assert_eq!(
        origin_summary.first_derivative_label.as_deref(),
        Some("jam chop 01.wav")
    );

    assert!(
        state
            .library
            .folder_browser
            .focus_file_across_sources_matching_tags(&child_sample, &state.metadata.tags_by_file)
    );
    let child_summary = state
        .selected_harvest_family_summary()
        .expect("child should have family summary");
    assert_eq!(child_summary.state_label, "New");
    assert_eq!(child_summary.origin_count, 1);
    assert_eq!(child_summary.derivative_count, 0);
    assert_eq!(child_summary.first_origin_label.as_deref(), Some("jam.wav"));

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::New,
        true,
    );
    state.show_selected_sample_harvest_origin(&mut ui::UiUpdateContext::default());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(origin_sample.to_string_lossy().as_ref())
    );
    assert!(
        state.ui.status.sample.contains("Showing harvest origin"),
        "{}",
        state.ui.status.sample
    );

    state.library.folder_browser.set_harvest_filter(
        crate::native_app::sample_library::folder_browser::model::HarvestFilter::HasDerivatives,
        true,
    );
    state.show_selected_sample_harvest_derivatives(&mut ui::UiUpdateContext::default());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(child_sample.to_string_lossy().as_ref())
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Showing harvest derivative"),
        "{}",
        state.ui.status.sample
    );
}

#[test]
fn harvest_family_summary_keeps_missing_origin_visible_for_child() {
    let origin_root = tempfile::tempdir().expect("origin root");
    let derived_root = tempfile::tempdir().expect("derived root");
    let origin_sample = origin_root.path().join("missing-origin.wav");
    let child_sample = derived_root.path().join("missing-origin chop.wav");
    fs::write(&origin_sample, [0_u8; 8]).expect("write origin");
    fs::write(&child_sample, [0_u8; 8]).expect("write child");
    let origin_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-missing-origin-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let child_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-missing-origin-child-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let origin_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            origin_id.clone(),
            PathBuf::from("missing-origin.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(1),
        content_hash: None,
    };
    let child_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            child_id.clone(),
            PathBuf::from("missing-origin chop.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(2),
        content_hash: None,
    };
    wavecrate::sample_sources::library::record_harvest_derivation(
        &wavecrate::sample_sources::NewHarvestDerivation {
            parent: origin_identity,
            child: child_identity,
            operation: wavecrate::sample_sources::HarvestDerivationOperation::Extract,
            source_range: None,
            output_duration_seconds: None,
            destination_folder: Some(derived_root.path().to_path_buf()),
            inherited_metadata: wavecrate::sample_sources::HarvestMetadataSnapshot::default(),
            tool_version: String::from("wavecrate-test"),
        },
    )
    .expect("record edge");
    fs::remove_file(&origin_sample).expect("remove origin after graph recording");
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new_with_id(
                origin_id,
                origin_root.path().to_path_buf(),
            ),
            wavecrate::sample_sources::SampleSource::new_with_id(
                child_id,
                derived_root.path().to_path_buf(),
            ),
        ]);
    assert!(
        state
            .library
            .folder_browser
            .focus_file_across_sources_matching_tags(&child_sample, &state.metadata.tags_by_file)
    );

    let summary = state
        .selected_harvest_family_summary()
        .expect("child should keep harvest family summary");

    assert_eq!(summary.origin_count, 1);
    assert_eq!(summary.missing_origin_count, 1);
    assert_eq!(
        summary.first_origin_label.as_deref(),
        Some("missing-origin.wav")
    );
    assert_eq!(summary.derivative_count, 0);
}

#[test]
fn harvest_family_summary_keeps_missing_derivative_visible_for_origin() {
    let origin_root = tempfile::tempdir().expect("origin root");
    let derived_root = tempfile::tempdir().expect("derived root");
    let origin_sample = origin_root.path().join("missing-derivative.wav");
    let child_sample = derived_root.path().join("missing-derivative chop.wav");
    fs::write(&origin_sample, [0_u8; 8]).expect("write origin");
    fs::write(&child_sample, [0_u8; 8]).expect("write child");
    let origin_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-missing-derivative-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let child_id = wavecrate::sample_sources::SourceId::from_string(format!(
        "harvest-missing-derivative-child-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let origin_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            origin_id.clone(),
            PathBuf::from("missing-derivative.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(1),
        content_hash: None,
    };
    let child_identity = wavecrate::sample_sources::HarvestFileIdentity {
        key: wavecrate::sample_sources::HarvestFileKey::new(
            child_id.clone(),
            PathBuf::from("missing-derivative chop.wav"),
        ),
        file_size: Some(8),
        modified_ns: Some(2),
        content_hash: None,
    };
    wavecrate::sample_sources::library::record_harvest_derivation(
        &wavecrate::sample_sources::NewHarvestDerivation {
            parent: origin_identity,
            child: child_identity,
            operation: wavecrate::sample_sources::HarvestDerivationOperation::Extract,
            source_range: None,
            output_duration_seconds: None,
            destination_folder: Some(derived_root.path().to_path_buf()),
            inherited_metadata: wavecrate::sample_sources::HarvestMetadataSnapshot::default(),
            tool_version: String::from("wavecrate-test"),
        },
    )
    .expect("record edge");
    fs::remove_file(&child_sample).expect("remove child after graph recording");
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new_with_id(
                origin_id,
                origin_root.path().to_path_buf(),
            ),
            wavecrate::sample_sources::SampleSource::new_with_id(
                child_id,
                derived_root.path().to_path_buf(),
            ),
        ]);
    state
        .library
        .folder_browser
        .select_file(origin_sample.display().to_string());

    let summary = state
        .selected_harvest_family_summary()
        .expect("origin should keep harvest family summary");

    assert_eq!(summary.origin_count, 0);
    assert_eq!(summary.derivative_count, 1);
    assert_eq!(summary.missing_derivative_count, 1);
    assert_eq!(
        summary.first_derivative_label.as_deref(),
        Some("missing-derivative chop.wav")
    );
}

#[test]
fn collection_context_menu_paints_collection_cleanup_action() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Collection,
        path: PathBuf::new(),
        source_id: None,
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: wavecrate::sample_sources::SampleCollection::new(0),
        sample_missing: false,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Collection 1"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Clear all broken files"));
    assert!(!frame.paint_plan.contains_text("Clean missing entry"));
    assert!(!frame.paint_plan.contains_text("Move to Trash"));
}

#[test]
fn missing_sample_context_menu_paints_cleanup_actions_without_file_actions() {
    let menu = crate::native_app::test_support::context_menu::BrowserContextMenu {
        kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Sample,
        path: PathBuf::from("C:\\Samples\\missing.wav"),
        source_id: None,
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: wavecrate::sample_sources::SampleCollection::new(0),
        sample_missing: true,
        anchor: Point::new(72.0, 142.0),
        title: String::from("missing.wav"),
    };
    let frame = crate::native_app::test_support::context_menu::browser_context_menu_overlay(&menu)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Copy Path"));
    assert!(frame.paint_plan.contains_text("Clean missing entry"));
    assert!(
        frame
            .paint_plan
            .contains_text("Clean all missing in collection")
    );
    assert!(!frame.paint_plan.contains_text("Reveal in Explorer"));
    assert!(!frame.paint_plan.contains_text("Move to Trash"));
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
    fs::write(parent.join("kicks").join("kick.wav"), [0_u8; 8]).expect("write test audio");

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
