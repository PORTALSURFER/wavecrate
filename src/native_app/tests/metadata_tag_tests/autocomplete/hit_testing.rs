use super::*;
use radiant::runtime::SurfaceFrame;

#[test]
fn metadata_autocomplete_does_not_block_sidebar_button_clicks() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata.tag_draft = String::from("ki");

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let input_rect = metadata_tag_text_input(&frame)
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let toggle_rect = tag_library_toggle_rect(&runtime.frame_with_default_theme(), input_rect)
        .expect("tag library toggle should paint");
    let point = toggle_rect.center();

    runtime.dispatch_primary_click(point);

    assert!(
        runtime.bridge().state().metadata.tag_library_open,
        "autocomplete popup must not prevent clicking the sidebar tag editor button"
    );
}

fn tag_library_toggle_rect(frame: &SurfaceFrame, _tag_input_rect: Rect) -> Option<Rect> {
    frame.paint_plan.first_svg_rect_for_widget(
        crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID,
    )
}

#[test]
fn metadata_autocomplete_does_not_block_folder_tree_clicks() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let expandable_child = source_root.path().join("Child Folder");
    fs::create_dir_all(expandable_child.join("Nested")).expect("expandable child folder");
    fs::write(expandable_child.join("Nested").join("nested.wav"), []).expect("nested child sample");
    let selected_file = source_root.path().join("tag-target.wav");
    fs::write(&selected_file, []).expect("sample file");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(selected_file.display().to_string());
    state
        .metadata
        .tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata.tag_draft = String::from("ki");
    let (clicked_folder_id, _initially_expanded) = state
        .library
        .folder_browser
        .first_visible_child_folder_expansion_for_tests()
        .expect("visible child folder with expander");

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 900.0));
    let frame = runtime.frame_with_default_theme();
    let input_rect = metadata_tag_text_input(&frame)
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let frame = runtime.frame_with_default_theme();
    let clicked_folder_label = std::path::Path::new(&clicked_folder_id)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("clicked folder should have a display label");
    let folder_row_id =
        crate::native_app::app_chrome::library_browser::library_sidebar::folder_row_widget_id(
            &clicked_folder_id,
        );
    let folder_row_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::Text(text)
                if text.widget_id == folder_row_id
                    && text.text.as_str() == clicked_folder_label =>
            {
                Some(text.rect)
            }
            _ => None,
        })
        .expect("folder row label should paint");
    let point = folder_row_rect.center();
    let target = runtime.widget_at(point);

    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_folder_id(),
        Some(clicked_folder_id.as_str()),
        "autocomplete popup must not prevent clicking folder row {clicked_folder_label}; target={target:?}, row={folder_row_id}, rect={folder_row_rect:?}"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_tag_library_clicks() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known.wav"),
        vec![String::from("kick"), String::from("bass")],
    );
    state.metadata.tag_draft = String::from("ki");
    state.metadata.tag_library_open = true;

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let input_rect = metadata_tag_text_input(&frame)
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let tag_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("bass")
        .expect("available tag should paint");
    let point = tag_rect.center();

    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .metadata
            .tags_by_file
            .get(&selected_file),
        Some(&vec![String::from("bass")]),
        "autocomplete popup must not prevent clicking tags in the tag library"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_source_row_clicks_with_tag_library_open() {
    let source_base = tempfile::tempdir().expect("source base");
    let first_root = source_base.path().join("Alpha Samples");
    let second_root = source_base.path().join("Beta Samples");
    fs::create_dir_all(&first_root).expect("first source");
    fs::create_dir_all(&second_root).expect("second source");
    fs::write(first_root.join("alpha.wav"), []).expect("first sample");
    fs::write(second_root.join("beta.wav"), []).expect("second sample");

    let mut state = gui_state_for_span_tests();
    let first_source = wavecrate::sample_sources::SampleSource::new(first_root.clone());
    let second_source = wavecrate::sample_sources::SampleSource::new(second_root.clone());
    let second_source_id = second_source.id.clone();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            first_source,
            second_source,
        ]);
    let first_file = first_root.join("alpha.wav").display().to_string();
    state.library.folder_browser.select_file(first_file);
    state
        .metadata
        .tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata.tag_draft = String::from("ki");
    state.metadata.tag_library_open = true;

    let mut runtime = native_runtime_for_tests(state, Vector2::new(589.0, 571.0));
    let frame = runtime.frame_with_default_theme();
    let input_rect = metadata_tag_text_input(&frame)
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let source_row_id =
        crate::native_app::app_chrome::library_browser::library_sidebar::source_row_widget_id(
            second_source_id.as_str(),
        );
    let source_rect = runtime
        .frame_with_default_theme()
        .layout
        .rects
        .get(&source_row_id)
        .copied()
        .expect("second source should paint");
    let point = source_rect.center();
    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_folder_path(),
        Some(second_root),
        "autocomplete popup and tag library must not prevent clicking source rows"
    );
}
