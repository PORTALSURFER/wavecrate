use super::identity::retained_folder_row_input_id;
use super::rows::{
    FOLDER_TREE_EMPTY_LABEL, FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA,
    FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH, folder_row, folder_tree_label_color,
    folder_tree_palette_for_tests, folder_tree_selected_hover_marker,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_full_palette;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::VisibleFolder;
use crate::native_app::sample_library::folder_browser::view_contract::TREE_ROW_HEIGHT;
use radiant::prelude as ui;
use radiant::prelude::IntoView;
use radiant::widgets::TextInputMessage;

#[test]
fn folder_tree_uses_shared_grey_sidebar_hover_fill() {
    let theme = ui::ThemeTokens::default();
    let palette = folder_tree_palette_for_tests(&theme);
    let expected = sidebar_row_full_palette(&theme);

    assert_eq!(palette.hovered, expected.hovered);
    assert_eq!(palette.candidate_hovered, expected.candidate_hovered);
    assert_eq!(palette.selected, expected.selected);
    assert_eq!(palette.selected_hovered, expected.selected_hovered);
}

#[test]
fn folder_tree_selected_hover_marker_uses_left_orange_rail() {
    let marker = folder_tree_selected_hover_marker();

    assert_eq!(marker.parts.width, FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH);
    assert_eq!(
        marker.color,
        ui::ThemeTokens::default()
            .accent_mint
            .with_alpha(FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA)
    );
}

#[test]
fn empty_folder_rows_use_subdued_idle_label_color() {
    let folder = visible_folder_for_tests(true);

    assert_eq!(
        folder_tree_label_color(&folder),
        Some(FOLDER_TREE_EMPTY_LABEL)
    );
}

#[test]
fn non_empty_folder_rows_use_default_idle_label_color() {
    let folder = visible_folder_for_tests(false);

    assert_eq!(folder_tree_label_color(&folder), None);
}

#[test]
fn selected_empty_folder_rows_keep_subdued_label_color() {
    let mut folder = visible_folder_for_tests(true);
    folder.selected = true;

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

    assert_eq!(
        frame.paint_plan.first_text_color("Folder"),
        Some(FOLDER_TREE_EMPTY_LABEL)
    );
}

#[test]
fn selected_non_empty_folder_rows_use_default_label_color() {
    let mut folder = visible_folder_for_tests(false);
    folder.selected = true;

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

    assert_eq!(
        frame.paint_plan.first_text_color("Folder"),
        Some(ui::ThemeTokens::default().text_primary)
    );
}

#[test]
fn focused_unselected_folder_rows_use_default_label_color() {
    let mut folder = visible_folder_for_tests(false);
    folder.focused = true;

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

    assert_eq!(
        frame.paint_plan.first_text_color("Folder"),
        Some(ui::ThemeTokens::default().text_primary)
    );
}

#[test]
fn focused_unselected_folder_rows_paint_selected_fill() {
    let mut folder = visible_folder_for_tests(false);
    folder.focused = true;

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));
    let selected_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
        .selected
        .expect("folder tree selected fill");

    assert!(
        frame.paint_plan.fill_rects().any(|fill| {
            fill.color == selected_fill && (fill.rect.height() - TREE_ROW_HEIGHT).abs() < 0.5
        }),
        "focused folder rows should paint the same base fill as selected source rows"
    );
}

#[test]
fn focused_selected_folder_rows_paint_selected_fill_without_hover_marker() {
    let mut folder = visible_folder_for_tests(false);
    folder.focused = true;
    folder.selected = true;

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));
    let selected_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
        .selected
        .expect("folder tree selected fill");
    let selected_hover_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
        .selected_hovered
        .expect("folder tree selected-hover fill");
    let marker = folder_tree_selected_hover_marker();

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == selected_fill),
        "focused selected folder rows should paint the base selected fill"
    );
    assert!(
        !frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == selected_hover_fill),
        "focused selected folder rows should reserve selected-hover fill for pointer hover"
    );
    assert!(
        !frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == marker.color && fill.rect.width() == marker.parts.width),
        "focused selected folder rows should reserve the selected-hover marker for pointer hover"
    );
}

#[test]
fn rename_folder_rows_project_draft_into_stable_input() {
    let mut folder = visible_folder_for_tests(false);
    folder.rename_draft = Some(String::from("Renamed Folder"));
    folder.rename_input_id = Some(4_242);

    let frame = folder_row(&folder)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));
    let input = frame
        .paint_plan
        .first_text_input()
        .expect("rename row should project a text input");

    assert_eq!(input.widget_id, 4_242);
    assert_eq!(input.state.value, "Renamed Folder");
    assert_eq!(input.state.selection_anchor, 0);
    assert_eq!(input.state.caret, "Renamed Folder".chars().count());
    assert!(!frame.paint_plan.contains_text("Folder"));
}

#[test]
fn rename_folder_rows_route_input_messages_to_folder_browser() {
    let mut folder = visible_folder_for_tests(false);
    folder.rename_draft = Some(String::from("Folder"));
    folder.rename_input_id = Some(4_242);
    let message = TextInputMessage::Changed {
        value: String::from("Renamed Folder"),
    };

    assert_eq!(
        folder_row(&folder)
            .view_dispatch_widget_output(4_242, ui::WidgetOutput::typed(message.clone()),),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::RenameInput(message)
        ))
    );
}

#[test]
fn standard_folder_rows_derive_stable_input_id_from_row_key() {
    let folder = visible_folder_for_tests(false);
    let input_id = retained_folder_row_input_id(folder.id.as_str());
    let mut surface = folder_row(&folder).into_surface();
    let bounds = ui::Rect::from_size(220.0, TREE_ROW_HEIGHT);
    let position = ui::Point::new(8.0, 10.0);

    surface.dispatch_widget_input(
        input_id,
        bounds,
        ui::WidgetInput::PointerPress {
            position,
            button: ui::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let output = surface.dispatch_widget_input(
        input_id,
        bounds,
        ui::WidgetInput::PointerRelease {
            position,
            button: ui::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_cloned::<GuiMessage>()),
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ActivateFolder(folder.id.clone(), Default::default())
        ))
    );
}

fn visible_folder_for_tests(empty: bool) -> VisibleFolder {
    VisibleFolder {
        id: String::from("folder"),
        name: String::from("Folder"),
        depth: 0,
        is_source_root: false,
        has_children: false,
        empty,
        locked: false,
        lock_inherited: false,
        expanded: false,
        selected: false,
        focused: false,
        drag_active: false,
        drag_source: false,
        drop_candidate: false,
        drop_target: false,
        drop_target_active: false,
        rename_draft: None,
        rename_input_id: None,
    }
}
