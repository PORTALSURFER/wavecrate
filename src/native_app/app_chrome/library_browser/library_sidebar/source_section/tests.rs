use super::identity::{AUTOMATION_SOURCE_ADD_BUTTON_ID, retained_source_row_input_id};
use super::rows::{
    SOURCE_ADD_BUTTON_HEIGHT, SOURCE_ADD_BUTTON_WIDTH, SOURCE_ROW_HEIGHT,
    SOURCE_ROW_LABEL_PADDING_X, source_add_button, source_add_button_tooltip_for_tests,
    source_missing_color_for_tests, source_role_icon_color_for_tests, source_row,
    source_row_outline_for_tests,
};
use super::source_selector;
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
use wavecrate::sample_sources::SourceRole;

fn test_source(id: &str) -> SourceEntry {
    SourceEntry::new(id, "Source", std::path::PathBuf::from("C:/samples"))
}

macro_rules! assert_no_left_source_marker {
    ($frame:expr) => {
        assert!(
            !$frame.paint_plan.fill_rects().any(|fill| {
                fill.rect.min.x <= SOURCE_ROW_LABEL_PADDING_X + 12.0
                    && fill.rect.width() <= 10.0
                    && fill.rect.height() <= 10.0
            }),
            "source rows should not paint a separate left color marker"
        );
    };
}

#[test]
fn source_add_button_routes_add_source_message() {
    assert_eq!(
        source_add_button(false).view_dispatch_widget_output(
            AUTOMATION_SOURCE_ADD_BUTTON_ID,
            ui::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
    );
}

#[test]
fn source_add_button_uses_regular_icon_button_chrome() {
    let frame = source_add_button(false).view_frame_at_size_with_default_theme(ui::Vector2::new(
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
fn source_add_button_exposes_tooltip_when_help_tooltips_are_enabled() {
    let surface = source_add_button(true).into_surface();
    let tooltip = surface
        .find_widget(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(tooltip, Some(source_add_button_tooltip_for_tests()));
}

#[test]
fn source_add_button_omits_tooltip_when_help_tooltips_are_disabled() {
    let surface = source_add_button(false).into_surface();
    let tooltip = surface
        .find_widget(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(tooltip, None);
}

#[test]
fn source_selector_threads_help_tooltips_to_add_button() {
    let source = test_source("source-with-tooltip");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, true);
    let surface = source_selector(&model).into_surface();
    let tooltip = surface
        .find_widget(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(tooltip, Some(source_add_button_tooltip_for_tests()));
}

#[test]
fn source_row_routes_primary_activation_through_interactive_row() {
    let source = test_source("source-a");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
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
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
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
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, SOURCE_ROW_HEIGHT));
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
fn source_rows_use_slim_outlined_item_chrome() {
    let source = test_source("source-bordered");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, SOURCE_ROW_HEIGHT));
    let outline = source_row_outline_for_tests();
    let stroke = frame
        .paint_plan
        .stroke_rects_for_widget(retained_source_row_input_id(source.id.as_str()))
        .find(|stroke| stroke.color == outline.color)
        .expect("source rows should paint a subtle item outline");

    assert_eq!(
        SOURCE_ROW_HEIGHT, 22.0,
        "source rows should stay slimmer than the old 24px baseline"
    );
    assert_eq!(stroke.width, outline.width);
    assert_eq!(stroke.rect.min.x, outline.inset);
    assert_eq!(stroke.rect.min.y, outline.inset);
    assert_eq!(stroke.rect.max.y, SOURCE_ROW_HEIGHT - outline.inset);
    assert_ne!(
        outline.color,
        sidebar_row_selected_fill_for_tests(),
        "source item outlines should not recreate the old selected rectangle"
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
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, SOURCE_ROW_HEIGHT));

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
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, SOURCE_ROW_HEIGHT));
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

#[test]
fn missing_source_row_paints_missing_badge_without_left_marker() {
    let mut source = test_source("source-missing");
    source.mark_missing_for_tests();
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(200.0, SOURCE_ROW_HEIGHT));

    assert!(
        frame.paint_plan.contains_text("MISSING"),
        "missing sources should get an explicit source-list badge"
    );
    assert_eq!(
        frame.paint_plan.first_text_color("Source"),
        Some(source_missing_color_for_tests()),
        "missing source labels should use warning text"
    );
    assert_eq!(
        frame.paint_plan.first_text_color("MISSING"),
        Some(source_missing_color_for_tests()),
        "missing source badges should use warning text"
    );
    assert_no_left_source_marker!(frame);
}

#[test]
fn primary_source_row_uses_role_icon_instead_of_text_badge() {
    let mut source = test_source("source-primary");
    source.role = SourceRole::Primary;
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(200.0, SOURCE_ROW_HEIGHT));
    let icon_rect = frame
        .paint_plan
        .svgs()
        .next()
        .expect("primary source icon")
        .rect;

    assert!(icon_rect.height() <= SOURCE_ROW_HEIGHT);
    assert_eq!(
        source_role_icon_color_for_tests(),
        ui::Rgba8::new(255, 255, 255, 255),
        "source role icons should use a white tint"
    );
    assert!(
        !frame.paint_plan.contains_text("PRI"),
        "primary sources should not render the old text badge"
    );
    assert_no_left_source_marker!(frame);
}

#[test]
fn protected_source_row_uses_role_icon_instead_of_text_badge() {
    let mut source = test_source("source-protected");
    source.role = SourceRole::Protected;
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(200.0, SOURCE_ROW_HEIGHT));
    let icon_rect = frame
        .paint_plan
        .svgs()
        .next()
        .expect("protected source icon")
        .rect;

    assert!(icon_rect.height() <= SOURCE_ROW_HEIGHT);
    assert_eq!(
        source_role_icon_color_for_tests(),
        ui::Rgba8::new(255, 255, 255, 255),
        "source role icons should use a white tint"
    );
    assert!(
        !frame.paint_plan.contains_text("PRO") && !frame.paint_plan.contains_text("PROT"),
        "protected sources should not render the old text badge"
    );
    assert_no_left_source_marker!(frame);
}

#[test]
fn normal_source_row_keeps_role_slot_neutral() {
    let source = test_source("source-normal");
    let state = FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let row = model.rows.first().expect("source row");
    let frame = source_row(row)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(200.0, SOURCE_ROW_HEIGHT));

    assert_eq!(
        frame.paint_plan.svgs().count(),
        0,
        "normal sources should not paint a role icon"
    );
    assert!(
        !frame.paint_plan.contains_text("PRI")
            && !frame.paint_plan.contains_text("PRO")
            && !frame.paint_plan.contains_text("PROT"),
        "normal sources should stay free of source-role text badges"
    );
    assert_no_left_source_marker!(frame);
}

#[test]
fn source_selector_header_reports_missing_sources() {
    let mut missing = test_source("source-missing");
    missing.mark_missing_for_tests();
    let present = SourceEntry::new(
        "source-present",
        "Present",
        std::path::PathBuf::from("C:/present"),
    );
    let state = FolderBrowserState::from_sources_deferred(
        vec![missing.clone(), present],
        missing.id.clone(),
    );
    let model = SourceSelectorViewModel::from_folder_browser(&state, false);
    let frame = source_selector(&model)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, 76.0));

    assert!(
        frame.paint_plan.contains_text("Sources (1 missing)"),
        "source header should expose missing source count"
    );
    assert_eq!(
        frame.paint_plan.first_text_color("Sources (1 missing)"),
        Some(source_missing_color_for_tests()),
        "source header should use warning text when any source is missing"
    );
}
