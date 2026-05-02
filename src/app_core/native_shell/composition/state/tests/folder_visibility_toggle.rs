use super::*;
use crate::compat_app_contract::FolderPaneIdModel;

#[test]
fn folder_visibility_toggle_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .folder_visibility_toggle_button_rect(&layout, &model)
        .expect("folder visibility button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_header_action_at_point(&layout, &model, point),
        Some(UiAction::ToggleShowAllFolders {
            pane: Some(FolderPaneIdModel::Upper),
        })
    );
}

#[test]
fn folder_visibility_toggle_button_uses_compact_square_layout() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = populated_sidebar_model();
    model.sources.show_all_items = false;
    let state = NativeShellState::new();

    let button = state
        .folder_visibility_toggle_button_rect(&layout, &model)
        .expect("folder visibility button should render");
    let style = style_for_layout(&layout);

    assert!((button.width() - button.height()).abs() <= 0.5);
    assert!(button.height() <= style.sizing.sidebar_action_button_height + 0.5);
}

#[test]
fn folder_flatten_toggle_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = populated_sidebar_model();
    model.sources.flattened_view = true;
    let mut state = NativeShellState::new();
    let button = state
        .folder_flatten_toggle_button_rect(&layout, &model)
        .expect("folder flatten button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_header_action_at_point(&layout, &model, point),
        Some(UiAction::ToggleFolderFlattenedView {
            pane: Some(FolderPaneIdModel::Upper),
        })
    );
}

#[test]
fn folder_header_renders_both_square_toggle_buttons_without_overlap() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let state = NativeShellState::new();
    let visibility = state
        .folder_visibility_toggle_button_rect(&layout, &model)
        .expect("folder visibility button should render");
    let flatten = state
        .folder_flatten_toggle_button_rect(&layout, &model)
        .expect("folder flatten button should render");

    assert!((visibility.width() - visibility.height()).abs() <= 0.5);
    assert!((flatten.width() - flatten.height()).abs() <= 0.5);
    assert!(visibility.max.x <= flatten.min.x);
}
