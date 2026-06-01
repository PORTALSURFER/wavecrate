use super::super::*;

#[test]
fn metadata_autocomplete_suffix_is_not_editable_input_text() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let input_id = runtime
        .frame(&radiant::theme::ThemeTokens::default())
        .paint_plan
        .first_text_input()
        .map(|input| input.widget_id)
        .expect("metadata tag input should paint");
    assert!(runtime.focus_widget(input_id));

    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::KeyPress(WidgetKey::Backspace)),
        Some(input_id)
    );
    assert_eq!(runtime.bridge().state().metadata_tag_draft, "k");
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::KeyPress(WidgetKey::Backspace)),
        Some(input_id)
    );
    assert!(runtime.bridge().state().metadata_tag_draft.is_empty());
    assert!(!runtime.bridge().state().metadata_tag_completion_active());

    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let tag_input = frame
        .paint_plan
        .text_inputs()
        .find(|input| input.widget_id == input_id)
        .expect("metadata tag input should still paint");
    assert!(tag_input.state.value.is_empty());
    assert_eq!(tag_input.state.caret, 0);
    assert_eq!(tag_input.state.selection_anchor, 0);
    assert!(!frame.paint_plan.contains_text("ick"));
}

#[test]
fn metadata_autocomplete_does_not_block_sidebar_button_clicks() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .first_text_input()
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let toggle_rect = tag_library_toggle_rect(
        &runtime.frame(&radiant::theme::ThemeTokens::default()),
        input_rect,
    )
    .expect("tag library toggle should paint");
    let point = toggle_rect.center();

    runtime.dispatch_primary_click(point);

    assert!(
        runtime.bridge().state().metadata_tag_library_open,
        "autocomplete popup must not prevent clicking the sidebar tag editor button"
    );
}

fn tag_library_toggle_rect(frame: &ui::SurfaceFrame, tag_input_rect: Rect) -> Option<Rect> {
    frame.paint_plan.svgs().find_map(|svg| {
        (svg.rect.max.y <= tag_input_rect.min.y && svg.rect.min.x > tag_input_rect.min.x)
            .then_some(svg.rect)
    })
}

#[test]
fn metadata_autocomplete_does_not_block_folder_tree_clicks() {
    let mut state = gui_state_for_span_tests();
    let selected_file = state
        .folder_browser
        .selected_audio_files()
        .first()
        .expect("default browser should expose audio files")
        .id
        .clone();
    state.folder_browser.select_file(selected_file);
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");
    let selected_folder = state
        .folder_browser
        .selected_folder_path()
        .expect("selected folder")
        .display()
        .to_string();

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .first_text_input()
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let (label, folder_rect) = frame
        .paint_plan
        .text_runs()
        .find_map(|text| {
            text.text
                .as_str()
                .starts_with("[-] ")
                .then(|| (text.text.to_string(), text.rect))
        })
        .expect("expanded selected root folder should paint");
    let point = folder_rect.center();

    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .folder_browser
            .folder_expansion_for_tests(&selected_folder),
        Some(false),
        "autocomplete popup must not prevent clicking folder row {label}"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_tag_library_clicks() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known.wav"),
        vec![String::from("kick"), String::from("bass")],
    );
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_library_open = true;

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .first_text_input()
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let tag_rect = runtime
        .frame(&radiant::theme::ThemeTokens::default())
        .paint_plan
        .first_text_rect("bass")
        .expect("available tag should paint");
    let point = tag_rect.center();

    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .metadata_tags_by_file
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
    state.folder_browser = super::super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(first_root.clone()),
        wavecrate::sample_sources::SampleSource::new(second_root.clone()),
    ]);
    let first_file = first_root.join("alpha.wav").display().to_string();
    state.folder_browser.select_file(first_file);
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_library_open = true;

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(589.0, 571.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .first_text_input()
        .map(|input| input.rect)
        .expect("metadata tag input should paint");
    let input_point = input_rect.center();
    runtime.dispatch_primary_click(input_point);
    assert!(runtime.focused_widget().is_some());

    let source_rect = runtime
        .frame(&radiant::theme::ThemeTokens::default())
        .paint_plan
        .first_text_rect("Beta Samples")
        .expect("second source should paint");
    let point = source_rect.center();
    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime
            .bridge()
            .state()
            .folder_browser
            .selected_folder_path(),
        Some(second_root),
        "autocomplete popup and tag library must not prevent clicking source rows"
    );
}
