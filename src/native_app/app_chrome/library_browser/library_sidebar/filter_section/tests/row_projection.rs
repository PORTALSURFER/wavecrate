use super::*;

#[test]
fn filter_section_projects_name_text_input() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);

    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    let input = frame
        .paint_plan
        .first_text_input()
        .expect("name filter should project a text input");

    assert_eq!(input.widget_id, NAME_FILTER_INPUT_ID);
    assert_eq!(input.state.value, "");
    assert_eq!(
        input.placeholder.as_ref().map(|value| value.as_str()),
        Some("Any")
    );
    assert!(
        !frame
            .paint_plan
            .contains_text_after_x("Any", input.rect.min.x),
        "name filter should not paint Any as a read-only property value"
    );
}

#[test]
fn filter_section_projects_tag_text_input_with_row_labels() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);

    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    let inputs = frame.paint_plan.text_inputs().collect::<Vec<_>>();

    assert!(frame.paint_plan.contains_text("Name"));
    assert!(frame.paint_plan.contains_text("Tags"));
    assert!(frame.paint_plan.contains_text("Curat"));
    assert!(frame.paint_plan.contains_text("Harve"));
    assert!(frame.paint_plan.contains_text("Type"));
    assert!(frame.paint_plan.contains_text("Ratin"));
    assert!(
        !frame.paint_plan.contains_text("Curate")
            && !frame.paint_plan.contains_text("Harvest")
            && !frame.paint_plan.contains_text("Rating")
    );
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.widget_id)
            .collect::<Vec<_>>(),
        vec![NAME_FILTER_INPUT_ID, TAG_FILTER_INPUT_ID]
    );
}

#[test]
fn filter_section_filter_name_labels_are_compact_and_same_size() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    let labels = ["Name", "Tags", "Curat", "Harve", "Type", "Ratin"];
    let label_runs = labels
        .iter()
        .map(|label| {
            frame
                .paint_plan
                .first_text_run(label)
                .unwrap_or_else(|| panic!("missing filter label {label}"))
        })
        .collect::<Vec<_>>();

    assert!(label_runs.iter().all(|run| run.text.len() <= 5));
    assert!(
        label_runs
            .iter()
            .all(|run| run.font_size == label_runs[0].font_size)
    );
}

#[test]
fn filter_section_filter_labels_dispatch_family_enable_changes() {
    let mut state = FolderBrowserState::load_default();
    state.apply_message(FolderBrowserMessage::NameFilterInput(
        TextInputMessage::Changed {
            value: String::from("kick"),
        },
    ));
    state.set_rating_filter(1, true);
    let model = FilterSectionViewModel::from_folder_browser(&state, false);

    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_filter_family_label_toggle_id("Name"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetFilterFamilyEnabled(FilterFamily::Name, false)
        ))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_filter_family_label_toggle_id("Ratin"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetFilterFamilyEnabled(FilterFamily::Rating, false)
        ))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_filter_family_label_toggle_id("Tags"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: true }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetFilterFamilyEnabled(FilterFamily::Tags, true)
        ))
    );
}

#[test]
fn filter_section_projects_curation_scope_dropdown_and_dispatches_changes() {
    let mut state = FolderBrowserState::load_default();
    state.set_curation_scope(BrowserCurationScope::Ratings, true);
    let mut model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert!(
        frame
            .paint_plan
            .first_widget_rect(CURATION_FILTER_DROPDOWN_TRIGGER_ID)
            .is_some()
    );
    assert!(frame.paint_plan.contains_text("Rate  v"));
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            CURATION_FILTER_DROPDOWN_TRIGGER_ID,
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::ToggleCurationFilterDropdown)
    );

    model.curation.dropdown_open = true;
    let open_frame = filter_section(&model).view_frame_at_size_with_default_theme(
        ui::Vector2::new(240.0, FILTER_SECTION_TEST_FRAME_HEIGHT),
    );
    assert!(
        open_frame
            .paint_plan
            .first_widget_rect(CURATION_FILTER_DROPDOWN_TRIGGER_ID)
            .is_some()
    );
    assert_eq!(
        open_frame
            .paint_plan
            .first_widget_rect(automation_curation_filter_dropdown_option_id("All")),
        None,
        "curation menu options should render in the sidebar overlay, not inside the clipped filter section"
    );

    let overlay_frame = curation_filter_dropdown_overlay(&model, 4.0, 167.0)
        .expect("open curation dropdown should project an overlay menu");
    let overlay_frame = overlay_frame.view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    let all_rect = overlay_frame
        .paint_plan
        .first_widget_rect(automation_curation_filter_dropdown_option_id("All"))
        .expect("All option should render in the overlay");
    let rate_rect = overlay_frame
        .paint_plan
        .first_widget_rect(automation_curation_filter_dropdown_option_id("Rate"))
        .expect("Rate option should render in the overlay");
    let tags_rect = overlay_frame
        .paint_plan
        .first_widget_rect(automation_curation_filter_dropdown_option_id("Tags"))
        .expect("Tags option should render in the overlay");
    assert!(all_rect.min.x > 0.0);
    assert!(
        (120.0..=180.0).contains(&all_rect.width()),
        "curation overlay options should stay fixed to the compact sidebar control width, got {}",
        all_rect.width()
    );
    assert!(all_rect.max.x <= 240.0);
    assert!(rate_rect.min.y > all_rect.min.y);
    assert!(tags_rect.min.y > rate_rect.min.y);
    assert_eq!(
        curation_filter_dropdown_overlay(&model, 4.0, 167.0)
            .expect("open curation dropdown should project an overlay menu")
            .view_dispatch_widget_output(
                automation_curation_filter_dropdown_option_id("Tags"),
                ui::WidgetOutput::typed(ButtonMessage::Activate),
            ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetCurationScope(BrowserCurationScope::Tags, true)
        ))
    );
}

#[test]
fn filter_section_projects_harvest_family_toggle_button() {
    let state = FolderBrowserState::load_default();
    let mut model = FilterSectionViewModel::from_folder_browser(&state, false);
    model.harvest.family_available = true;
    model.harvest.family_open = true;
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            HARVEST_FAMILY_TOGGLE_ID,
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::ToggleHarvestFamilyPanel)
    );
}

#[test]
fn filter_section_projects_harvest_filter_dropdown_and_dispatches_changes() {
    let mut state = FolderBrowserState::load_default();
    state.set_harvest_filter(HarvestFilter::NeedsReview, true);
    let mut model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert!(
        frame
            .paint_plan
            .first_widget_rect(HARVEST_FILTER_DROPDOWN_TRIGGER_ID)
            .is_some()
    );
    assert!(frame.paint_plan.contains_text("Needs Review  v"));
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            HARVEST_FILTER_DROPDOWN_TRIGGER_ID,
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::ToggleHarvestFilterDropdown)
    );

    model.harvest.dropdown_open = true;
    let open_frame = filter_section(&model).view_frame_at_size_with_default_theme(
        ui::Vector2::new(240.0, FILTER_SECTION_TEST_FRAME_HEIGHT),
    );
    assert_eq!(
        open_frame
            .paint_plan
            .first_widget_rect(automation_harvest_filter_dropdown_option_id("Needs Review")),
        None,
        "harvest menu options should render in the sidebar overlay, not inside the clipped filter section"
    );

    let overlay_frame = harvest_filter_dropdown_overlay(&model, 4.0, 167.0)
        .expect("open harvest dropdown should project an overlay menu");
    let overlay_frame = overlay_frame.view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    for (label, filter) in [
        ("Needs Review", HarvestFilter::NeedsReview),
        ("Done", HarvestFilter::Done),
        ("Ignored", HarvestFilter::Ignored),
        ("All", HarvestFilter::All),
    ] {
        assert!(
            overlay_frame
                .paint_plan
                .first_widget_rect(automation_harvest_filter_dropdown_option_id(label))
                .is_some(),
            "{label} option should render in the overlay"
        );
        assert_eq!(
            harvest_filter_dropdown_overlay(&model, 4.0, 167.0)
                .expect("open harvest dropdown should project an overlay menu")
                .view_dispatch_widget_output(
                    automation_harvest_filter_dropdown_option_id(label),
                    ui::WidgetOutput::typed(ButtonMessage::Activate),
                ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::SetHarvestFilter(filter, true)
            ))
        );
    }
}

#[test]
fn filter_section_hides_clear_buttons_when_filters_are_empty() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);

    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert_eq!(
        frame
            .paint_plan
            .first_widget_rect(name_filter_clear_button_id()),
        None
    );
    assert_eq!(
        frame
            .paint_plan
            .first_widget_rect(tag_filter_clear_button_id()),
        None
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            name_filter_clear_button_id(),
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        None
    );
}

#[test]
fn filter_section_projects_name_clear_button_for_active_name_filter() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel {
        name_filter: String::from("kick"),
        ..FilterSectionViewModel::from_folder_browser(&state, false)
    };

    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert!(
        frame
            .paint_plan
            .first_widget_rect(name_filter_clear_button_id())
            .is_some()
    );
    assert_eq!(
        frame
            .paint_plan
            .first_widget_rect(tag_filter_clear_button_id()),
        None
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            name_filter_clear_button_id(),
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::NameFilterInput(empty_filter_message())
        ))
    );
}

#[test]
fn filter_section_projects_tag_clear_button_for_active_tag_filter() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel {
        tag_filter: String::from("drum"),
        ..FilterSectionViewModel::from_folder_browser(&state, false)
    };

    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert_eq!(
        frame
            .paint_plan
            .first_widget_rect(name_filter_clear_button_id()),
        None
    );
    assert!(
        frame
            .paint_plan
            .first_widget_rect(tag_filter_clear_button_id())
            .is_some()
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            tag_filter_clear_button_id(),
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::TagFilterInput(empty_filter_message())
        ))
    );
}

#[test]
fn filter_section_projects_playback_type_toggles_and_dispatches_changes() {
    let mut state = FolderBrowserState::load_default();
    state.set_playback_type_filter(PlaybackTypeFilter::Loop, true);
    let model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert!(
        frame
            .paint_plan
            .first_widget_rect(automation_playback_type_filter_toggle_id("1-Shot"))
            .is_some()
    );
    assert!(
        frame
            .paint_plan
            .first_widget_rect(automation_playback_type_filter_toggle_id("Loop"))
            .is_some()
    );
    assert!(frame.paint_plan.contains_text("1-Shot"));
    assert!(frame.paint_plan.contains_text("Loop"));
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_playback_type_filter_toggle_id("1-Shot"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: true }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::TogglePlaybackTypeFilter(PlaybackTypeFilter::OneShot, true)
        ))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_playback_type_filter_toggle_id("Loop"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::TogglePlaybackTypeFilter(PlaybackTypeFilter::Loop, false)
        ))
    );
}

#[test]
fn filter_section_projects_rating_toggles_and_dispatches_changes() {
    let mut state = FolderBrowserState::load_default();
    state.set_rating_filter(-3, true);
    state.set_rating_filter(0, true);
    let model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        240.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));

    assert!(
        frame
            .paint_plan
            .first_widget_rect(automation_rating_filter_toggle_id("T3"))
            .is_some()
    );
    assert!(
        frame
            .paint_plan
            .first_widget_rect(automation_rating_filter_toggle_id("U"))
            .is_some()
    );
    assert!(
        frame
            .paint_plan
            .first_widget_rect(automation_rating_filter_toggle_id("K4"))
            .is_some()
    );
    assert!(frame.paint_plan.fill_rects().any(|fill| {
        fill.color == rating_filter_swatch_color(-3, true)
            && fill.rect.width() == RATING_FILTER_SWATCH_SIZE as f32
    }));
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == rating_filter_swatch_color(1, false))
    );
    assert!(
        !frame
            .paint_plan
            .text_labels()
            .any(|label| matches!(label, "T3" | "T2" | "T1" | "U" | "K1" | "K2" | "K3" | "K4"))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_rating_filter_toggle_id("K4"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: true }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ToggleRatingFilter(4, true)
        ))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_rating_filter_toggle_id("U"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ToggleRatingFilter(0, false)
        ))
    );
    assert_eq!(
        filter_section(&model).view_dispatch_widget_output(
            automation_rating_filter_toggle_id("T3"),
            ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ToggleRatingFilter(-3, false)
        ))
    );
}
