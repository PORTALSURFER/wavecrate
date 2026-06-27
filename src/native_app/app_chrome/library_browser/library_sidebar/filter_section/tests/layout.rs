use super::*;

#[test]
fn filter_section_layout_uses_configured_height() {
    let mut state = FolderBrowserState::load_default();
    state.resize_filter_panel(ui::DragHandleMessage::started(ui::Point::new(0.0, 200.0)));
    state.resize_filter_panel(ui::DragHandleMessage::moved(ui::Point::new(0.0, 120.0)));
    let model = FilterSectionViewModel::from_folder_browser(&state, false);

    let layout = ui::column([
        filter_section(&model),
        ui::spacer().fill_width().fill_height(),
    ])
    .view_layout_at_size(ui::Vector2::new(240.0, 600.0));
    let section = layout
        .rects
        .get(&FILTER_SECTION_NODE_ID)
        .expect("filter section layout rect");

    assert_eq!(section.height(), state.filter_panel_height());
}

#[test]
fn filter_resize_header_uses_full_width_hit_target() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);
    let layout = filter_section(&model).view_layout_at_size(ui::Vector2::new(240.0, 120.0));
    let section = layout
        .rects
        .get(&FILTER_SECTION_NODE_ID)
        .expect("filter section layout rect");
    let header = layout
        .rects
        .get(&FILTER_RESIZE_HEADER_ID)
        .expect("filter resize header layout rect");
    let drag = ui::DragHandleMessage::started(ui::Point::new(header.center().x, header.center().y));

    assert!(
        header.width() >= section.width() - FILTER_PANEL_PADDING * 2.0,
        "filter resize header should span the useful panel width, section={section:?}, header={header:?}"
    );
    assert_eq!(
        filter_section(&model)
            .view_dispatch_widget_output(FILTER_RESIZE_HEADER_ID, ui::WidgetOutput::typed(drag),),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ResizeFilterPanel(drag)
        ))
    );
}

#[test]
fn filter_section_controls_scroll_when_panel_is_cramped() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel {
        panel_height: FILTER_PANEL_HEADER_HEIGHT + FILTER_PANEL_PADDING * 2.0 + 18.0,
        ..FilterSectionViewModel::from_folder_browser(&state, false)
    };

    let layout = ui::column([
        filter_section(&model),
        ui::spacer().fill_width().fill_height(),
    ])
    .view_layout_at_size(ui::Vector2::new(240.0, 600.0));
    let overflow = layout
        .overflow_flags
        .get(&FILTER_SECTION_SCROLL_NODE_ID)
        .expect("filter controls should have a scroll viewport");

    assert!(
        overflow.y,
        "cramped filter controls should scroll vertically"
    );
}

#[test]
fn filter_section_wide_labels_keep_harvest_controls_aligned_in_cramped_width() {
    let state = FolderBrowserState::load_default();
    let model = FilterSectionViewModel::from_folder_browser(&state, false);
    let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
        180.0,
        FILTER_SECTION_TEST_FRAME_HEIGHT,
    ));
    let harvest_label = frame
        .paint_plan
        .first_widget_rect(automation_filter_family_label_toggle_id("Harvest"))
        .expect("Harvest label should render");
    let harvest_text = frame
        .paint_plan
        .first_text_run("Harvest")
        .expect("Harvest text should render")
        .rect;
    let harvest_dropdown = frame
        .paint_plan
        .first_widget_rect(HARVEST_FILTER_DROPDOWN_TRIGGER_ID)
        .expect("Harvest dropdown should render");

    assert_eq!(harvest_label.width(), FILTER_LABEL_WIDTH);
    assert!(
        harvest_text.min.x >= harvest_label.min.x && harvest_text.max.x <= harvest_label.max.x,
        "Harvest text should remain inside the widened label cell, label={harvest_label:?}, text={harvest_text:?}"
    );
    assert!(
        harvest_label.max.x <= harvest_dropdown.min.x,
        "widened Harvest label should not overlap the dropdown controls, label={harvest_label:?}, dropdown={harvest_dropdown:?}"
    );
    assert!(
        harvest_dropdown.width() > 0.0 && harvest_dropdown.max.x <= 180.0,
        "cramped Harvest dropdown should stay readable inside the sidebar, dropdown={harvest_dropdown:?}"
    );
}
