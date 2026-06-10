use super::super::*;

#[test]
fn folder_browser_metadata_hides_tag_entry_when_no_file_is_selected() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
            &browser,
            260.0,
            false,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
            &[],
            None,
        )
        .view_frame_at_size(Vector2::new(260.0, 620.0), &theme);

    assert!(frame.paint_plan.contains_text("Tags"));
    assert!(!frame.paint_plan.contains_text("Metadata"));
    assert!(!frame.paint_plan.contains_text("Tags (1)"));
    assert!(!frame.paint_plan.contains_text("kick"));
    assert!(metadata_tag_text_input(&frame).is_none());
}

#[test]
fn folder_browser_metadata_tags_grow_combined_entry_field() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    let small_tags = vec![String::from("kick")];
    let larger_tags = vec![
        String::from("kick"),
        String::from("warm"),
        String::from("one-shot"),
        String::from("distorted"),
    ];
    let small =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &small_tags,
            &[],
            None,
        )
        .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));
    let larger =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &larger_tags,
            &[],
            None,
        )
        .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    assert!(larger.paint_plan.contains_text("distorted"));
    assert!(!larger.paint_plan.contains_text("More"));
    assert!(frame_has_clip_height(&small, 24.0));
    let first_tag = larger
        .paint_plan
        .first_text_rect("kick")
        .expect("first tag should paint");
    let wrapped_tag = larger
        .paint_plan
        .first_text_rect("distorted")
        .expect("wrapped tag should paint");
    assert!(wrapped_tag.min.y > first_tag.min.y);
}

#[test]
fn folder_browser_metadata_tag_field_caps_at_six_rows_then_scrolls() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    let tags = (0..24)
        .map(|index| format!("tag-{index:02}"))
        .collect::<Vec<_>>();
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
            &[],
            None,
        )
        .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    let tag_clip = frame
        .paint_plan
        .clip_starts()
        .find_map(|clip| ((clip.rect.height() - 129.0).abs() < 0.01).then_some(clip.rect));
    assert!(
        tag_clip.is_some(),
        "combined tag field should clip overflowing tag rows"
    );
}

#[test]
fn metadata_tag_input_prompts_for_category_before_adding_new_tag() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert_eq!(
        state.metadata_tag_input_placeholder(),
        "select group/parent tag"
    );
    assert_eq!(state.ui.status.sample, "Choose a category for deep-kick");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("sound"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Sound Type")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("sound"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(state.metadata.tag_draft.is_empty());
    assert_eq!(state.ui.status.sample, "Added tag deep-kick");
}

#[test]
fn metadata_tag_category_selection_shows_all_options_immediately() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    let options = state.metadata_tag_completion_options();
    assert_eq!(
        options
            .iter()
            .map(|option| (option.tag.as_str(), option.selected))
            .collect::<Vec<_>>(),
        vec![
            ("Sound Type", true),
            ("Character", false),
            ("Prefix", false),
            ("Tuning/Scale", false),
        ]
    );
    assert!(state.metadata_tag_completion_active());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Character")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::new(),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("character")
    );
}

#[test]
fn metadata_tag_category_cancel_aborts_pending_tag_entry() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("sound"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert!(state.metadata_tag_completion_active());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::CancelMetadataTagEntry,
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(!state.metadata_tag_completion_active());
    assert_eq!(state.metadata_tag_input_placeholder(), "add tag");
    assert!(state.metadata.tag_draft.is_empty());
    assert!(state.metadata.tag_tokens.is_empty());
    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.metadata.tag_dictionary.get("deep-kick"), None);
}

#[test]
fn metadata_tag_category_invalid_completion_selection_keeps_enter_commit_available() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectMetadataTagCompletion(String::from(
            "Not a category",
        )),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert!(state.metadata_tag_completion_active());
    assert_eq!(state.ui.status.sample, "Choose a category for deep-kick");

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("sound"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
}

#[test]
fn metadata_tag_input_persists_tag_assignments_and_removals_to_source_database() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("persistent-tag.wav");
    fs::write(&sample_path, []).expect("sample file");
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("metadata-tags-persist-test"),
        source_root.path().to_path_buf(),
    );
    let source_id = source.id.as_str().to_string();
    wavecrate::sample_sources::config::save(&crate::native_app::test_support::AppConfig {
        sources: vec![source.clone()],
        core: crate::native_app::test_support::AppSettingsCore::default(),
    })
    .expect("seed config");
    let selected_file = sample_path.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[source]);
    state
        .library
        .folder_browser
        .select_file(selected_file.clone());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick, Warm Tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick"), String::from("warm-tone")])
    );

    super::super::super::metadata::persist_metadata_tag_additions_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick"), String::from("warm-tone")],
    )
    .expect("persist tags");

    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path())
        .expect("open source db");
    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels"),
        vec![String::from("deep-kick"), String::from("warm-tone")]
    );

    super::super::super::metadata::persist_metadata_tag_removals_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick")],
    )
    .expect("persist tag removal");

    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels after removal"),
        vec![String::from("warm-tone")]
    );

    let mut reloaded = NativeAppState::load_default().expect("default state reloads");
    reloaded.refresh_persisted_metadata_tags_for_source(&source_id);
    assert_eq!(
        reloaded.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("warm-tone")])
    );
}

#[test]
fn metadata_tag_input_keeps_delimiters_while_editing() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("kick, warm tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata.tags_by_file.is_empty());
    assert_eq!(state.metadata.tag_draft, "kick, warm tone");
}

#[test]
fn metadata_tag_input_submits_typed_prefix_without_autoselecting_completion() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata_tag_completion_suffix().as_deref(),
        Some("ck")
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("ki"));
    assert_eq!(state.ui.status.sample, "Choose a category for ki");
}

#[test]
fn metadata_tag_completion_request_shows_suggestions_without_selecting_one() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("ki"));
    assert_eq!(state.ui.status.sample, "Choose a category for ki");
}

#[test]
fn metadata_tag_second_completion_request_activates_first_suggestion() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kick")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("kick")])
    );
}

#[test]
fn metadata_tag_input_arrows_through_multiple_known_prefix_matches() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![
            String::from("kick"),
            String::from("kicker"),
            String::from("kind"),
        ],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kick")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kicker")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kind")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("kind")])
    );
    assert!(state.metadata.tag_draft.is_empty());
}

#[test]
fn folder_browser_metadata_tag_field_renders_completion_suffix_without_overlay_options() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    let completion_options = vec![
        super::super::super::metadata::MetadataTagCompletionOption {
            tag: String::from("kick"),
            category: "Sound Type",
            selected: true,
        },
        super::super::super::metadata::MetadataTagCompletionOption {
            tag: String::from("kicker"),
            category: "Character",
            selected: false,
        },
    ];
    let theme = radiant::theme::ThemeTokens::default();
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
            &browser,
            260.0,
            true,
            "ki",
            &[String::from("kick")],
            None,
            "add tag",
            Some("ck"),
            completion_options.as_slice(),
            &[String::from("warm")],
            &[],
            None,
        )
        .view_frame_at_size(Vector2::new(260.0, 620.0), &theme);

    assert!(frame.paint_plan.contains_text("kick"));
    let tag_input = metadata_tag_text_input(&frame).expect("tag input should paint");
    assert_eq!(tag_input.state.value, "ki");
    assert_eq!(tag_input.state.selection_anchor, 2);
    assert_eq!(tag_input.state.caret, 2);
    assert_eq!(tag_input.completion_suffix.as_deref(), Some("ck"));
    assert_eq!(tag_input.completion_color, theme.text_muted);
    assert!(!frame.paint_plan.contains_text("Sound Type"));
    assert!(!frame.paint_plan.contains_text("kicker"));
    assert!(!frame.paint_plan.contains_text("Character"));
    assert!(!frame.paint_plan.contains_text("Tab kick"));
    assert!(frame.paint_plan.contains_text("warm"));
    assert!(
        frame
            .paint_plan
            .text_inputs()
            .any(|input| input.rect.height() <= 14.0)
    );
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| (fill.rect.height() - 18.0).abs() < 0.01)
    );
}

#[test]
fn folder_browser_metadata_category_completion_renders_above_tag_input() {
    let (mut baseline_state, _baseline_source_root, _baseline_selected_file) =
        native_app_state_with_temp_sample("baseline-tag-target.wav");
    let baseline_frame = crate::native_app::test_support::view(&mut baseline_state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    let baseline_tag_input =
        metadata_tag_text_input(&baseline_frame).expect("baseline tag input should paint");

    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("category-tag-target.wav");
    state.metadata.tag_input_mode =
        crate::native_app::test_support::MetadataTagInputMode::Category {
            pending_tag: String::from("new-tag"),
        };
    state.metadata.tag_draft.clear();

    let frame = crate::native_app::test_support::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    let tag_input = metadata_tag_text_input(&frame).expect("tag input should paint");
    let final_option = frame
        .paint_plan
        .first_text_rect("Tuning/Scale")
        .expect("final category option should paint");

    assert!(
        final_option.max.y <= tag_input.rect.min.y,
        "category completion popup should fit above the tag input, option {final_option:?}, input {:?}",
        tag_input.rect
    );
    assert_eq!(
        tag_input.rect.min.y, baseline_tag_input.rect.min.y,
        "floating category completion should not expand or shift the tags section"
    );
}
