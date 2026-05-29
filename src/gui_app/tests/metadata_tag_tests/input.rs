use super::super::*;

#[test]
fn folder_browser_metadata_hides_tag_entry_when_no_file_is_selected() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &theme,
    );

    assert!(!frame_has_text(&frame, "Metadata"));
    assert!(!frame_has_text(&frame, "Tags (1)"));
    assert!(!frame_has_text(&frame, "kick"));
    assert!(
        !frame
            .paint_plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::TextInput(_)))
    );
}

#[test]
fn folder_browser_metadata_tags_grow_combined_entry_field() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let small_tags = vec![String::from("kick")];
    let larger_tags = vec![
        String::from("kick"),
        String::from("warm"),
        String::from("one-shot"),
        String::from("distorted"),
    ];
    let small = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let larger = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&larger, "distorted"));
    assert!(!frame_has_text(&larger, "More"));
    assert!(frame_has_clip_height(&small, 24.0));
    let first_tag = text_rect(&larger, "kick").expect("first tag should paint");
    let wrapped_tag = text_rect(&larger, "distorted").expect("wrapped tag should paint");
    assert!(wrapped_tag.min.y > first_tag.min.y);
}

#[test]
fn folder_browser_metadata_tag_field_caps_at_six_rows_then_scrolls() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = (0..24)
        .map(|index| format!("tag-{index:02}"))
        .collect::<Vec<_>>();
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let tag_clip = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::ClipStart(clip) = primitive
            && (clip.rect.height() - 129.0).abs() < 0.01
        {
            return Some(clip.rect);
        }
        None
    });
    assert!(
        tag_clip.is_some(),
        "combined tag field should clip overflowing tag rows"
    );
}

#[test]
fn metadata_tag_input_prompts_for_category_before_adding_new_tag() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata_tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert_eq!(
        state.metadata_tag_input_placeholder(),
        "select group/parent tag"
    );
    assert_eq!(state.sample_status, "Choose a category for deep-kick");

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
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
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("sound"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata_tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(state.metadata_tag_draft.is_empty());
    assert_eq!(state.sample_status, "Added tag deep-kick");
}

#[test]
fn metadata_tag_input_persists_tag_assignments_and_removals_to_source_database() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("persistent-tag.wav");
    fs::write(&sample_path, []).expect("sample file");
    wavecrate::sample_sources::config::save(&super::super::super::AppConfig {
        sources: vec![wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )],
        core: super::super::super::AppSettingsCore::default(),
    })
    .expect("seed config");
    let selected_file = sample_path.display().to_string();
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.folder_browser.select_file(selected_file.clone());

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick, Warm Tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick"), String::from("warm-tone")])
    );

    super::super::super::metadata_tags::persist_metadata_tag_additions_for_tests(
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

    super::super::super::metadata_tags::persist_metadata_tag_removals_for_tests(
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

    let reloaded = GuiAppState::load_default().expect("default state reloads");
    assert_eq!(
        reloaded.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("warm-tone")])
    );
}

#[test]
fn metadata_tag_input_keeps_delimiters_while_editing() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("kick, warm tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tags_by_file.is_empty());
    assert_eq!(state.metadata_tag_draft, "kick, warm tone");
}

#[test]
fn metadata_tag_input_enters_selected_known_prefix() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
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

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("kick")])
    );
}

#[test]
fn metadata_tag_input_arrows_through_multiple_known_prefix_matches() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known-file"),
        vec![
            String::from("kick"),
            String::from("kicker"),
            String::from("kind"),
        ],
    );

    state.apply_message(
        super::super::super::GuiMessage::MetadataTagInput(
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
        Some("kick")
    );

    state.apply_message(
        super::super::super::GuiMessage::MoveMetadataTagCompletion(1),
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
        super::super::super::GuiMessage::MoveMetadataTagCompletion(1),
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
        super::super::super::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("kind")])
    );
    assert!(state.metadata_tag_draft.is_empty());
}

#[test]
fn folder_browser_metadata_tag_field_renders_completion_suffix_and_options() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let completion_options = vec![
        super::super::super::metadata_tags::MetadataTagCompletionOption {
            tag: String::from("kick"),
            category: "Sound Type",
            selected: true,
        },
        super::super::super::metadata_tags::MetadataTagCompletionOption {
            tag: String::from("kicker"),
            category: "Character",
            selected: false,
        },
    ];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &theme,
    );

    assert!(frame_has_text(&frame, "kick"));
    let tag_input = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input),
            _ => None,
        })
        .expect("tag input should paint");
    assert_eq!(tag_input.state.value, "ki");
    assert_eq!(tag_input.state.selection_anchor, 2);
    assert_eq!(tag_input.state.caret, 2);
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "ck")
    }));
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if (fill.rect.height() - 15.0).abs() < 0.01
                    && fill.color == theme.accent_mint.blend_toward(theme.bg_primary, 0.12)
        )
    }));
    assert!(frame_has_text(&frame, "Sound Type"));
    assert!(frame_has_text(&frame, "kicker"));
    assert!(frame_has_text(&frame, "Character"));
    assert!(!frame_has_text(&frame, "Tab kick"));
    assert!(frame_has_text(&frame, "warm"));
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::TextInput(input) if input.rect.height() <= 14.0
        )
    }));
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill) if (fill.rect.height() - 18.0).abs() < 0.01
        )
    }));
}
