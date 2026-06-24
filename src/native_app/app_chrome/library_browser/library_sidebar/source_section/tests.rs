use super::identity::{AUTOMATION_SOURCE_ADD_BUTTON_ID, retained_source_row_input_id};
use super::rows::{
    SOURCE_ADD_BUTTON_HEIGHT, SOURCE_ADD_BUTTON_WIDTH, SOURCE_ROW_LABEL_PADDING_X,
    source_add_button, source_row,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::{
    sidebar_row_hover_fill_for_tests, sidebar_row_palette_for_tests,
    sidebar_row_selected_fill_for_tests,
};
use crate::native_app::app_chrome::view_models::library_sidebar::SourceSelectorViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::{FolderBrowserState, model::SourceEntry};
use radiant::prelude as ui;
use radiant::prelude::IntoView;
use radiant::widgets::ButtonMessage;

fn test_source(id: &str) -> SourceEntry {
    SourceEntry::new(id, "Source", std::path::PathBuf::from("C:/samples"))
}

#[test]
fn source_add_button_routes_add_source_message() {
    assert_eq!(
        source_add_button().view_dispatch_widget_output(
            AUTOMATION_SOURCE_ADD_BUTTON_ID,
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
    );
}

#[test]
fn source_add_button_uses_regular_icon_button_chrome() {
    let frame = source_add_button().view_frame_at_size_with_default_theme(ui::Vector2::new(
        SOURCE_ADD_BUTTON_WIDTH,
        SOURCE_ADD_BUTTON_HEIGHT,
    ));
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .expect("source add button should paint a plus icon");

    assert!(
        !frame.paint_plan.contains_text("+"),
        "source add should not render as a text button"
    );
    assert!(icon_rect.width() <= SOURCE_ADD_BUTTON_WIDTH);
    assert!(icon_rect.height() <= SOURCE_ADD_BUTTON_HEIGHT);
}

#[test]
fn source_row_routes_primary_activation_through_interactive_row() {
    let source = test_source("source-a");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state);
    let row = model.rows.first().expect("source row");

    assert_eq!(
        source_row(row).view_dispatch_widget_output(
            retained_source_row_input_id(source.id.as_str()),
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SelectSource(source.id.clone())
        ))
    );
}

#[test]
fn source_row_routes_secondary_activation_to_context_menu() {
    let source = test_source("source-b");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let position = ui::Point::new(12.0, 20.0);
    let model = SourceSelectorViewModel::from_folder_browser(&state);
    let row = model.rows.first().expect("source row");

    assert_eq!(
        source_row(row).view_dispatch_widget_output(
            retained_source_row_input_id(source.id.as_str()),
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::SecondaryActivate { position }),
        ),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::OpenSourceContextMenu(source.id.clone(), position)
        ))
    );
}

#[test]
fn selected_source_row_paints_selected_highlight_without_left_active_marker() {
    let source = test_source("source-active");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state);
    let row = model.rows.first().expect("source row");
    let frame =
        source_row(row).view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, 24.0));
    let selected_fill = sidebar_row_palette_for_tests()
        .selected
        .expect("source selected fill");

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == selected_fill),
        "selected source should keep the orange selected highlight"
    );
    assert!(
        !frame.paint_plan.fill_rects().any(|fill| {
            fill.rect.width() <= 3.5 && fill.rect.min.x <= 4.5 && fill.rect.height() < 20.0
        }),
        "selected source should not paint a separate left active marker"
    );
}

#[test]
fn inactive_source_row_does_not_paint_active_marker() {
    let source = test_source("source-inactive");
    let selected = test_source("source-active");
    let state = FolderBrowserState::from_sources_deferred(
        vec![source.clone(), selected.clone()],
        selected.id.clone(),
    );
    let model = SourceSelectorViewModel::from_folder_browser(&state);
    let row = model.rows.first().expect("source row");
    let frame =
        source_row(row).view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, 24.0));

    assert!(
        !frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == sidebar_row_selected_fill_for_tests()),
        "inactive sources should stay visually quiet"
    );
}

#[test]
fn source_row_label_keeps_left_breathing_room() {
    let source = test_source("source-padded");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state);
    let row = model.rows.first().expect("source row");
    let frame =
        source_row(row).view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, 24.0));
    let label_rect = frame
        .paint_plan
        .first_text_rect("Source")
        .expect("source label");

    assert!(
        label_rect.min.x >= SOURCE_ROW_LABEL_PADDING_X,
        "source label should be inset from the sidebar edge: {label_rect:?}"
    );
}

#[test]
fn source_rows_use_shared_grey_sidebar_hover_fill() {
    assert_eq!(
        sidebar_row_palette_for_tests().hovered,
        Some(sidebar_row_hover_fill_for_tests())
    );
}
