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
